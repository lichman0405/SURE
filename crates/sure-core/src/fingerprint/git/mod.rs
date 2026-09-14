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

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
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

    /// What is put in front of *every* Git invocation, before the subcommand.
    ///
    /// These are not about the question being asked; they are about what a
    /// repository is allowed to make Git do while answering it. **A project is
    /// untrusted input by assumption** — it may have been written by a coding
    /// agent, or cloned from anywhere — and a repository carries its own
    /// configuration, which Git reads and obeys. Two of those settings name
    /// programs:
    ///
    /// - `core.fsmonitor=false` — the setting names a hook Git runs to ask which
    ///   paths changed. It is read from the repository being described, so a
    ///   repository can name a program and have Git start it. Describing a
    ///   project would then be the same act as running that project's code, and
    ///   SURE's whole premise is that it inspects a project without executing
    ///   it. Turning it off costs a slower `status` on repositories that use it.
    /// - `--no-pager` — `core.pager` names a program too. Git does not page into
    ///   a pipe, so this changes nothing today; it is here so that it cannot
    ///   start to matter if this output ever stops being one.
    ///
    /// The same reasoning is why the process gets `GIT_TERMINAL_PROMPT=0` and a
    /// null stdin further down: a Git that stops to ask a question is a check
    /// that hangs with no output, and a check with no output is indistinguishable
    /// from one that is still working.
    ///
    /// # This is not the whole of it, and saying so is the point
    ///
    /// An earlier version of this comment claimed these two settings bought the
    /// guarantee that fingerprinting a project cannot execute it. **They do
    /// not**, and a security review of that commit was right to say so.
    ///
    /// A content filter names a program as well, and it is the one of these that
    /// cannot be turned off from here. Its name is not fixed: a tracked
    /// `.gitattributes` says `*.psd filter=lfs`, and the command is a
    /// `filter.<name>.clean` setting in the repository's own configuration. Both
    /// halves come from the project, and the name is only known once Git has
    /// read the project — so there is nothing to pass `-c` for. Nor could it be
    /// overridden even if the name were known: with the filter off, Git would
    /// compare a file's raw bytes against a stored version that was written
    /// *through* the filter, and report every such file as changed. The answer
    /// would be wrong, which is worse than absent.
    ///
    /// So a repository that names one is refused instead — see
    /// [`Self::CONFIG_ARGUMENTS`] and [`FingerprintError::RepositoryRunsPrograms`].
    /// What that check covers, what it deliberately does not, and the four
    /// routes that were measured rather than reasoned about are in
    /// `docs/architecture/FINGERPRINTING.md`.
    ///
    /// Kept as a named constant rather than written inline so that a test can
    /// read it, the same way [`Self::STATUS_ARGUMENTS`] is read. A setting that
    /// quietly stopped being passed is exactly the change no test would
    /// otherwise notice, because its absence changes no output on any repository
    /// that does not exploit it.
    pub const SAFETY_ARGUMENTS: &'static [&'static str] =
        &["-c", "core.fsmonitor=false", "--no-pager"];

    /// How SURE asks what programs the repository itself is configured to run.
    ///
    /// Run before the status, because the status is the invocation that runs
    /// them. `docs/architecture/FINGERPRINTING.md` has the measurements: `status`
    /// runs a repository's filter on every path it has to compare, and
    /// `rev-parse` — the other invocation — does not.
    ///
    /// - `config` — the question.
    /// - `--list` — every setting, not one named in advance. SURE does not know
    ///   which filter a project uses, and the answer has to be found rather than
    ///   guessed.
    /// - `--includes` — redundant *today*, and kept on purpose. A repository
    ///   reaches a filter through `include.path` as easily as through its own
    ///   file, and the two cannot be told apart from the outside — but Git
    ///   follows includes whenever it searches all its files, so with no scope
    ///   named, `--list` already reads them. This was **measured rather than
    ///   assumed**, and the first version of this comment claimed the opposite:
    ///   an included filter was found with the flag and without it, and only
    ///   `--local` changes the answer. The flag stays because the property is
    ///   worth stating in the invocation rather than inheriting from a default,
    ///   and because a scope added later would silently stop includes being read
    ///   without it. `the_config_arguments_are_the_ones_the_module_doc_explains`
    ///   pins it so that removing it is a decision and not a tidy-up.
    /// - `-z` — no quoting and no line splitting. A setting's value is an
    ///   arbitrary program with arbitrary arguments in it, and reading that
    ///   back out of a format that escapes things means parsing Git's escaping
    ///   rather than the setting.
    ///
    /// The scope matters as much as the flags, and it is two environment
    /// settings rather than one: see [`without_the_machines_configuration`].
    pub const CONFIG_ARGUMENTS: &'static [&'static str] = &["config", "--list", "--includes", "-z"];

    /// The settings that name a program Git runs over the contents of a file.
    ///
    /// Matched as `filter.<anything>.<one of these>` on the *shape* and not on a
    /// list of driver names, because the name is the project's to choose. A
    /// check against known names — `lfs` and the handful of others that ship
    /// with something — is the check a project gets past by calling its filter
    /// something else.
    ///
    /// `clean` is the one measured to run during a status; `process` is the same
    /// machinery in its long-running form. `smudge` is here because the question
    /// this list answers is "does the repository declare a program to run over
    /// its files?", and the answer does not depend on which of SURE's commands
    /// happens to reach it today. Enumerating that per command is how the next
    /// command quietly runs one.
    const FILTER_PROGRAM_SUFFIXES: &'static [&'static str] = &["clean", "smudge", "process"];

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
        //
        // This is deliberately the *first* invocation, before the check below,
        // because it is the one of the three that was measured not to run a
        // filter: it reads the repository's index and not its working tree, so
        // there is nothing for a filter to convert. The order is therefore
        // checker, then the invocation that runs programs, and nothing runs
        // before the check.
        let prefix = self.prefix(root)?;

        // Before the status, and not after: the status is the invocation that
        // runs these, so a check that came afterwards would be a report.
        let programs = self.named_programs(root)?;
        if !programs.is_empty() {
            return Err(FingerprintError::RepositoryRunsPrograms {
                root: root.to_path_buf(),
                settings: programs,
            });
        }

        let output = self.stdout(
            root,
            "read the status of this working tree",
            Self::STATUS_ARGUMENTS,
            &[],
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
            &[],
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

    /// The programs this repository's own configuration tells Git to run over
    /// the project's files, as `key = value` for each, sorted.
    ///
    /// Empty when the repository names none, which is the ordinary case and the
    /// only case in which a fingerprint is taken.
    ///
    /// # Errors
    ///
    /// Whatever [`Self::stdout`] returns. A repository whose configuration SURE
    /// cannot read is not a repository SURE can say is free of them, so this is
    /// not a place to carry on with an empty answer.
    fn named_programs(&self, root: &Path) -> Result<Vec<String>, FingerprintError> {
        let environment = without_the_machines_configuration(root);
        let borrowed: Vec<(&str, &OsStr)> = environment
            .iter()
            .map(|(key, value)| (*key, value.as_os_str()))
            .collect();
        let output = self.stdout(
            root,
            "read what this repository is configured to run",
            Self::CONFIG_ARGUMENTS,
            &borrowed,
        )?;
        Ok(filter_programs(&output))
    }

    /// Run one Git command inside `root` and return what it wrote to stdout.
    ///
    /// `environment` is added to the child's environment, on top of the two
    /// settings every invocation gets. It is a parameter rather than something
    /// written at each call site so that the one invocation that needs it is
    /// visible as the one invocation that needs it.
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
        environment: &[(&str, &OsStr)],
    ) -> Result<Vec<u8>, FingerprintError> {
        let output = Command::new(&self.program)
            // Read-only, and it says so. Without this, `status` refreshes the
            // index — writing to the repository SURE was asked to look at, and
            // possibly failing on a read-only checkout or a locked index.
            // Checking a project must not change it. It also removes the index
            // refresh, and with it the `post-index-change` hook a repository
            // could otherwise have Git run while SURE is describing it.
            .arg("--no-optional-locks")
            // What a repository is not allowed to make Git do while answering,
            // before the subcommand. See [`Self::SAFETY_ARGUMENTS`].
            .args(Self::SAFETY_ARGUMENTS)
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
            .envs(environment.iter().copied())
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

/// The environment that hides the two configuration scopes the *machine*
/// supplies, leaving the invocation reading only what the repository supplies.
///
/// The question the check asks is narrow on purpose: **did this repository name
/// a program?** A `filter.lfs.clean = git-lfs clean -- %f` in the system
/// configuration is the user's own tool, installed for their own reasons, and
/// refusing every repository on a machine that has Git LFS would make SURE
/// useless on that machine. What matters is the configuration that travelled
/// with the project.
///
/// So both machine scopes are suppressed, and each for its own reason:
///
/// - `GIT_CONFIG_NOSYSTEM=1` — the documented switch for the system file. The
///   machine this was written on has `filter.lfs.*` in Git for Windows' own
///   system config, so without this the check would report it for every
///   repository on the machine.
/// - `GIT_CONFIG_GLOBAL` — pointed at a path that **cannot exist**, which is how
///   Git is told to read no global configuration. `NUL` on Windows and
///   `/dev/null` on Unix would both do, and neither is a name this workspace is
///   willing to hard-code for the other platform. The path chosen is
///   `<root>/.git/config/<name>`: `.git/config` is a *file* in every repository
///   Git makes, and is a file even in the two cases where `.git` itself is not a
///   directory — a linked worktree and a submodule — so nothing can ever exist
///   below it. A path that merely did not exist yet would be one a project could
///   create, and then a repository could add settings to the answer about
///   itself.
///
/// Git tolerates a missing global file rather than failing, which is not assumed
/// here: it was measured, along with the rest of this check's behaviour, and the
/// measurements are in `docs/architecture/FINGERPRINTING.md`.
fn without_the_machines_configuration(root: &Path) -> [(&'static str, OsString); 2] {
    let nowhere = root
        .join(".git")
        .join("config")
        .join("sure-no-global-configuration");
    [
        ("GIT_CONFIG_NOSYSTEM", OsString::from("1")),
        ("GIT_CONFIG_GLOBAL", nowhere.into_os_string()),
    ]
}

/// The `filter.<name>.<what>` settings in a repository's configuration, as
/// `key = value`, sorted.
///
/// The input is `git config --list --includes -z`: NUL between settings, and a
/// newline between a setting's key and its value. Splitting on the **first**
/// newline of each record rather than on the last is what makes a value that
/// contains one survive — a filter command is an arbitrary program with
/// arbitrary arguments, and a `--format` with a newline in it is a value, not
/// two settings.
///
/// Sorted so that the message a person reads is the same on every run, for the
/// same reason the fingerprint's own lists are sorted.
fn filter_programs(config: &[u8]) -> Vec<String> {
    let mut found = Vec::new();
    for record in config.split(|byte| *byte == 0) {
        let (key, value) = match record.iter().position(|byte| *byte == b'\n') {
            Some(at) => (&record[..at], &record[at + 1..]),
            None => (record, [].as_slice()),
        };
        let key = String::from_utf8_lossy(key);
        let Some(named) = key.strip_prefix("filter.") else {
            continue;
        };
        // From the right, because a driver name may itself contain dots: Git
        // writes the setting for `[filter "a.b"]` as `filter.a.b.clean`, and
        // only the last component is the one of these three.
        let Some((_, what)) = named.rsplit_once('.') else {
            continue;
        };
        if !Git::FILTER_PROGRAM_SUFFIXES.contains(&what) {
            continue;
        }
        found.push(format!("{key} = {}", String::from_utf8_lossy(value)));
    }
    found.sort();
    found
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
///
/// # Why this also refuses a path that climbs
///
/// The caller does `root.join(relative)` and then reads what is there, so a
/// `..` in `relative` is a read outside the project — `root.join("../../etc")`
/// is not inside `root`, and `join` will not say so. Nothing here relies on Git
/// having rejected such a path: a repository is a thing SURE is asked to check,
/// its `.git/index` is a file in it, and an index Git will read is not the same
/// as an index Git would have written. `strip_prefix` is a textual operation
/// and passes `..` straight through, so the refusal is its own step.
///
/// The check runs even when the project *is* the repository root and there is no
/// prefix to strip, because that is the case with no strip at all between a path
/// Git printed and a path that gets opened.
fn relative_to_root<'a>(path: &'a Path, prefix: &Path) -> Result<&'a Path, FingerprintError> {
    let outside = || FingerprintError::OutsideRoot {
        path: path.to_path_buf(),
    };
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(outside());
    }
    if prefix.as_os_str().is_empty() {
        return Ok(path);
    }
    path.strip_prefix(prefix).map_err(|_| outside())
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
    /// Something that is neither a file nor a directory nor a link, named by its
    /// kind: a pipe, a socket, a device.
    ///
    /// The kind is recorded rather than any contents, because there are none to
    /// read and because opening it is what would not return. See the arm in
    /// [`Reader::read`] that produces this.
    Special(&'static str),
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
            Self::Special(kind) => {
                digest.field("special").field(kind);
            }
            Self::Gone => {
                digest.field("gone");
            }
        }
    }
}

/// What kind of not-a-file something is, as a name that goes in a digest.
///
/// The names are this module's own and are never printed to anybody, so they are
/// stable across platforms on purpose: `fifo` means the same state on Linux and
/// on macOS, and a digest taken on one is the digest the other would take. On
/// Windows a pipe is not a filesystem entry and Git cannot report one as a
/// tracked path, so the fall-through is what Windows returns.
fn file_kind(file_type: &fs::FileType) -> &'static str {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt as _;
        if file_type.is_fifo() {
            return "fifo";
        }
        if file_type.is_socket() {
            return "socket";
        }
        if file_type.is_char_device() {
            return "char-device";
        }
        if file_type.is_block_device() {
            return "block-device";
        }
    }
    #[cfg(not(unix))]
    let _ = file_type;
    "other"
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
        if !metadata.is_file() {
            // Neither a file, nor a directory, nor a link: a pipe, a socket, a
            // device. **This is answered without opening it, because opening it
            // is the thing that does not return.** `File::open` on a FIFO with
            // no writer blocks until one appears, and SURE reading a project
            // must not be stoppable by the project: a repository that tracks
            // `f` and whose working tree has a FIFO at `f` would otherwise hang
            // a check with no output and no way to tell it from a slow one.
            //
            // What is recorded is the kind, and not "no contents". A file
            // replaced by a pipe is a change, and a pipe replaced by a socket is
            // a change; recording both as `Gone` would make them one state. This
            // is the same decision as [`Contents::Link`] — describe what is
            // there, do not read through it — and it is why a special file named
            // by Git is *covered* while one met inside a walked directory is a
            // loss the walk refuses on. The walk cannot tell what else a
            // directory it could not fully read contains; Git has named exactly
            // one path, and its kind is knowable without opening it.
            self.take_file(relative)?;
            return Ok(Contents::Special(file_kind(&metadata.file_type())));
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A path that climbs out is refused however the prefix reads.
    ///
    /// This is a unit test of the rule rather than an end-to-end reproduction,
    /// and that is worth stating: no `git` on this machine will print a status
    /// line containing `..`, so the only way to reach this through the public
    /// entry point would be to hand-write a `.git/index` Git accepts — which is
    /// exactly the input the guard is for, and is not something a test can
    /// cheaply construct. The alternative to testing the rule directly was
    /// testing nothing and calling the guard covered.
    #[test]
    fn a_path_that_climbs_is_refused_with_or_without_a_prefix() {
        for prefix in [Path::new(""), Path::new("app")] {
            for climbing in ["../outside", "app/../../outside", "/etc/passwd"] {
                match relative_to_root(Path::new(climbing), prefix) {
                    Err(FingerprintError::OutsideRoot { path }) => {
                        assert_eq!(path, Path::new(climbing));
                    }
                    other => panic!(
                        "{climbing:?} under prefix {prefix:?} gave {other:?}, and joining that \
                         onto a project root would leave it"
                    ),
                }
            }
        }
    }

    /// The ordinary cases still work, so the refusal above is not a refusal of
    /// everything.
    #[test]
    fn a_path_inside_the_prefix_is_still_accepted() {
        assert_eq!(
            relative_to_root(Path::new("app/src/main.rs"), Path::new("app")).unwrap(),
            Path::new("src/main.rs")
        );
        // The project *is* the repository root: no prefix, and the path comes
        // back as it was.
        assert_eq!(
            relative_to_root(Path::new("src/main.rs"), Path::new("")).unwrap(),
            Path::new("src/main.rs")
        );
        // A single `.` component is not a climb, and refusing it would refuse a
        // path Git can legitimately print.
        assert_eq!(
            relative_to_root(Path::new("./src/main.rs"), Path::new("")).unwrap(),
            Path::new("./src/main.rs")
        );
    }

    /// A project in `app` is not confused by a sibling named `app-old`.
    #[test]
    fn the_prefix_comes_off_as_path_components_and_not_as_text() {
        match relative_to_root(Path::new("app-old/file.rs"), Path::new("app")) {
            Err(FingerprintError::OutsideRoot { .. }) => {}
            other => panic!("a string prefix would have answered {other:?}"),
        }
    }

    /// The bytes `git config --list --includes -z` writes, from settings.
    ///
    /// NUL between settings and a newline between a setting's key and its
    /// value — measured, not read off a manual, and the two are one character
    /// apart from the other obvious arrangement.
    fn config_bytes(settings: &[(&str, &str)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (key, value) in settings {
            out.extend_from_slice(key.as_bytes());
            out.push(b'\n');
            out.extend_from_slice(value.as_bytes());
            out.push(0);
        }
        out
    }

    #[test]
    fn every_shape_of_filter_setting_that_can_run_a_program_is_reported() {
        // The shape and not a list of known driver names, because the name is
        // the project's to choose: a check against `lfs` and the handful of
        // others that ship with something is a check a project gets past by
        // calling its filter something else.
        let found = filter_programs(&config_bytes(&[
            ("filter.probe.clean", "probe -- %f"),
            ("filter.probe.smudge", "probe -- %f"),
            ("filter.probe.process", "probe -- %f"),
            // Git writes `[filter "a.b"]` as `filter.a.b.clean`, so the driver
            // name can contain the separator the shape is matched on.
            ("filter.a.b.clean", "probe -- %f"),
            ("filter.whatever.smudge", ""),
        ]));
        assert_eq!(
            found,
            vec![
                "filter.a.b.clean = probe -- %f",
                "filter.probe.clean = probe -- %f",
                "filter.probe.process = probe -- %f",
                "filter.probe.smudge = probe -- %f",
                "filter.whatever.smudge = ",
            ],
            "sorted, and every one of them present"
        );
    }

    #[test]
    fn a_setting_that_names_no_program_is_not_reported() {
        // The other half of the rule, and the half that decides whether an
        // ordinary project can be fingerprinted at all. `filter.lfs.required`
        // ships in Git for Windows' system configuration, and a check that
        // refused on the word `filter` would refuse everything.
        let found = filter_programs(&config_bytes(&[
            ("filter.lfs.required", "true"),
            ("core.fsmonitor", "true"),
            ("core.pager", "less"),
            ("diff.probe.textconv", "probe"),
            ("merge.probe.driver", "probe %O %A %B"),
            // One character past the end of a suffix is a different setting.
            ("filter.probe.cleanish", "probe"),
            ("filter.probe", "probe"),
            // `filter.` has to be the start of the key, not somewhere inside it.
            ("core.filter.probe.clean", "probe"),
            ("user.name", "probe"),
        ]));
        assert_eq!(found, Vec::<String>::new(), "{found:?}");
    }

    #[test]
    fn a_setting_whose_value_is_empty_is_still_a_setting() {
        // `[filter "probe"] clean =` is a setting Git will read, and what Git
        // does with an empty command is not something this module is willing to
        // bet a project's safety on. The question asked is whether the
        // repository declared a program, and declaring one badly is declaring
        // one.
        assert_eq!(
            filter_programs(&config_bytes(&[("filter.probe.clean", "")])),
            vec!["filter.probe.clean = "]
        );
    }

    #[test]
    fn a_command_with_a_newline_in_it_survives_being_read_back() {
        // The record is `key\nvalue`, so the key is everything before the
        // **first** newline and the value is everything after. Splitting on the
        // last would make this two settings, the first of them a key that does
        // not exist — and the setting it really is would go unreported.
        let found = filter_programs(&config_bytes(&[(
            "filter.probe.clean",
            "probe --format=one\ntwo -- %f",
        )]));
        assert_eq!(
            found,
            vec!["filter.probe.clean = probe --format=one\ntwo -- %f"]
        );
    }

    #[test]
    fn the_variable_that_hides_the_machines_configuration_is_the_documented_one() {
        // Two switches, and each is the only one Git offers for its scope. A
        // change to either name would not fail anything else: the check would
        // simply start reporting the machine's own filters as the project's, or
        // stop reporting the project's.
        let environment = without_the_machines_configuration(Path::new("/work/project"));
        let keys: Vec<&str> = environment.iter().map(|(key, _)| *key).collect();
        assert_eq!(keys, vec!["GIT_CONFIG_NOSYSTEM", "GIT_CONFIG_GLOBAL"]);

        let nowhere = &environment[1].1;
        // A path under `.git/config`, which is a *file* in every repository Git
        // makes — including the two where `.git` itself is not a directory, a
        // linked worktree and a submodule. So nothing can exist there, which is
        // the whole of why this path and not any other: a path that merely did
        // not exist yet is one a project could create, and then a repository
        // could add settings to the answer about itself.
        let tail = Path::new(".git")
            .join("config")
            .join("sure-no-global-configuration");
        assert!(
            Path::new(nowhere).ends_with(&tail),
            "{nowhere:?} does not end with {tail:?}"
        );
    }
}

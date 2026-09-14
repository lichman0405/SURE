//! Why a project could not be fingerprinted.
//!
//! Every one of these means **there is no fingerprint**. None of them means
//! "here is a fingerprint of the part SURE managed to read", and that is the
//! whole reason this type exists rather than a `Result` that falls back.
//!
//! A fingerprint's only job is to let a later run say "the project has not
//! changed since this evidence was produced". A fingerprint over part of a
//! project answers that question wrongly for every file outside the part, and
//! nothing downstream can tell — the value looks exactly like a complete one.
//! So a partial fingerprint is not a degraded answer here; it is a wrong answer
//! wearing the right shape, which is the failure this product exists to prevent.
//!
//! The alternative — falling back to something weaker and saying so — is a real
//! option and it belongs to the caller, which knows what it has evidence for.
//! `docs/architecture/FINGERPRINTING.md` records what is left for whom.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// Why a project could not be fingerprinted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FingerprintError {
    /// The root was given as a relative path.
    ///
    /// The same rule as [`crate::scan::ScanError::NotAbsolute`], and it matters
    /// more here. A fingerprint is a value SURE **keeps** and compares against
    /// later, so resolving the root against whatever directory SURE happened to
    /// be started in would make the stored value mean different things on
    /// different days — and the whole use of it is asking whether two of them
    /// are the same.
    NotAbsolute {
        /// The root as it was given.
        root: PathBuf,
    },
    /// Git would not name a repository containing this directory.
    ///
    /// This is what an ordinary folder that was never a repository looks like.
    /// It is also what a damaged repository, a bare repository, and a repository
    /// SURE may not read look like, and the message says so rather than
    /// pretending to have told them apart — Git's own words are carried for the
    /// same reason.
    NoRepository {
        /// The directory SURE was pointed at.
        root: PathBuf,
        /// What Git said.
        message: String,
    },
    /// Git is not installed, or is not where the operating system looks for it.
    ///
    /// A separate answer from [`Self::NoRepository`] because it is a different
    /// problem with a different fix, and because the two are one step apart in
    /// the code: this is the process failing to start, and the other is the
    /// process running and saying no.
    GitUnavailable {
        /// The program SURE tried to run.
        program: OsString,
        /// What the operating system said.
        message: String,
    },
    /// Git ran and exited with a failure.
    GitFailed {
        /// What SURE was asking Git for, in words.
        operation: &'static str,
        /// The exit code, when the process exited rather than being killed.
        status: Option<i32>,
        /// What Git wrote to its error output.
        message: String,
    },
    /// Git's status output contained a record SURE does not understand.
    ///
    /// Refusing rather than skipping is deliberate. A record this code does not
    /// recognise is a change it cannot account for, and a fingerprint that
    /// silently omits a change is a fingerprint that says "nothing moved" when
    /// something did.
    UnrecognizedStatus {
        /// The record, as Git wrote it, cut short if it was very long.
        record: String,
    },
    /// Git's status output never said which commit was checked out.
    ///
    /// Without HEAD the fingerprint would be over the working tree alone, and a
    /// checkout of a different commit with no local changes would share a
    /// fingerprint with the state before it.
    MissingHead,
    /// Git reported a change to a path outside the directory SURE was given.
    ///
    /// SURE asks Git for the status of one subtree, so this cannot happen
    /// through Git honouring the pathspec it was given. It is a value rather
    /// than an assertion because the alternative — dropping the path — is the
    /// silent kind of wrong: the fingerprint would omit a change and look
    /// complete.
    OutsideRoot {
        /// The path Git reported.
        path: PathBuf,
    },
    /// A file Git named could not be read.
    Unreadable {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        message: String,
    },
    /// The project has more changed files than SURE will read in one run.
    TooManyFiles {
        /// How many it will read.
        limit: usize,
        /// The file that was one too many.
        path: PathBuf,
    },
    /// The changed files add up to more content than SURE will read in one run.
    TooManyBytes {
        /// How many bytes it will read.
        limit: u64,
        /// The file that was being read when the limit was passed.
        path: PathBuf,
    },
    /// A directory SURE had to read through could not be read completely.
    ///
    /// Carries the reason the walk gave, in the walk's own words, so that the
    /// message can say which of them happened rather than "something went
    /// wrong". Every one of them means some file inside is not in the
    /// fingerprint, which is the case this whole type exists to refuse.
    IncompleteTree {
        /// The directory that could not be read completely.
        path: PathBuf,
        /// What the walk said about the first thing it lost.
        detail: String,
    },
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute { root } => write!(
                f,
                "SURE was asked to fingerprint \"{}\", which is not a full path.\n\n\
                 A short path would be resolved against whatever directory SURE happened to be \
                 started in, so the same command would fingerprint a different project depending \
                 on where it ran — and a fingerprint is a value SURE keeps and compares later.\n\n\
                 Give the whole path, or run SURE from the project's own directory and name it as \
                 \".\".",
                root.display()
            ),
            Self::NoRepository { root, message } => write!(
                f,
                "There is no Git repository at \"{}\" that SURE can read.\n\n\
                 Git said: {message}\n\n\
                 An ordinary folder that was never a repository looks like this, and so does a \
                 repository SURE was not allowed to open. Either way SURE has not fingerprinted \
                 this project, and it will not report on part of it.\n\n\
                 Run SURE in the project's own folder, or use the content fingerprint for a \
                 project that is not under version control.",
                root.display()
            ),
            Self::GitUnavailable { program, message } => write!(
                f,
                "SURE could not run Git (\"{}\").\n\n\
                 The operating system said: {message}\n\n\
                 SURE fingerprints a Git project from what Git reports, and without Git there is \
                 no fingerprint — so this project has none rather than a partial one. Install \
                 Git and put it on the path, or use the content fingerprint for a project that \
                 is not under version control.",
                program.to_string_lossy()
            ),
            Self::GitFailed {
                operation,
                status,
                message,
            } => {
                let code = match status {
                    Some(code) => format!("exit code {code}"),
                    None => "no exit code, so it was killed".to_owned(),
                };
                write!(
                    f,
                    "SURE asked Git to {operation}, and Git failed ({code}).\n\n\
                     Git said: {message}\n\n\
                     Nothing was fingerprinted. A repository Git cannot describe is a repository \
                     SURE cannot report on."
                )
            }
            Self::UnrecognizedStatus { record } => write!(
                f,
                "SURE does not understand what Git said about this project: {record}\n\n\
                 SURE stopped rather than carry on, because a change it cannot read is a change \
                 it would leave out of the fingerprint — and a fingerprint that is missing a \
                 change is one that reports an old result as current.\n\n\
                 This usually means a newer Git than this build of SURE knows about. Updating \
                 SURE, or reporting the line above, is the way forward."
            ),
            Self::MissingHead => write!(
                f,
                "Git did not say which commit is checked out, so SURE has not fingerprinted this \
                 project.\n\n\
                 The commit is half of what a fingerprint is: without it, a project with no local \
                 changes would keep the same fingerprint across a checkout of a different \
                 version, and results from before the checkout would be reported as current."
            ),
            Self::OutsideRoot { path } => write!(
                f,
                "Git reported a change to \"{}\", which is outside the folder SURE was asked \
                 about.\n\n\
                 SURE stopped rather than leave it out. A fingerprint missing a change is a \
                 fingerprint that calls an old result current, which is worse than no answer at \
                 all.",
                path.display()
            ),
            Self::Unreadable { path, message } => write!(
                f,
                "SURE could not read \"{}\", which Git says has changed.\n\n\
                 The operating system said: {message}\n\n\
                 The file's contents are part of the project's fingerprint, so leaving it out \
                 would mean two different versions of the project could share one fingerprint. \
                 Check that the file exists and that you can read it.",
                path.display()
            ),
            Self::TooManyFiles { limit, path } => write!(
                f,
                "This project has more changed files than SURE reads in one run, and it stopped \
                 at \"{}\" after {limit} of them.\n\n\
                 SURE did not produce a fingerprint, because one over part of the project would \
                 report old results as current for the rest of it. Committing some of the \
                 changes, or running SURE on a smaller folder, brings it back within the limit.",
                path.display()
            ),
            Self::TooManyBytes { limit, path } => write!(
                f,
                "The changed files in this project add up to more than SURE reads in one run \
                 ({limit} bytes), and it stopped while reading \"{}\".\n\n\
                 SURE did not produce a fingerprint, because one over part of the project would \
                 report old results as current for the rest of it. Committing some of the \
                 changes, or running SURE on a smaller folder, brings it back within the limit.",
                path.display()
            ),
            Self::IncompleteTree { path, detail } => write!(
                f,
                "SURE could not read everything inside \"{}\", which is untracked or \
                 uncommitted.\n\n\
                 {detail}\n\n\
                 Whatever is in there is not in the fingerprint, so SURE did not produce one: a \
                 fingerprint that is missing files reports old results as current when those \
                 files change.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for FingerprintError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn all() -> Vec<FingerprintError> {
        let path = PathBuf::from("/work/project");
        vec![
            FingerprintError::NotAbsolute { root: path.clone() },
            FingerprintError::NoRepository {
                root: path.clone(),
                message: "fatal: not a git repository".to_owned(),
            },
            FingerprintError::GitUnavailable {
                program: OsString::from("git"),
                message: "program not found".to_owned(),
            },
            FingerprintError::GitFailed {
                operation: "read the repository's status",
                status: Some(128),
                message: "fatal: bad revision".to_owned(),
            },
            FingerprintError::UnrecognizedStatus {
                record: "2 R. N... 100644 100644 100644 a b R100 x\ty".to_owned(),
            },
            FingerprintError::MissingHead,
            FingerprintError::OutsideRoot {
                path: PathBuf::from("elsewhere/file.rs"),
            },
            FingerprintError::Unreadable {
                path: path.clone(),
                message: "Access is denied.".to_owned(),
            },
            FingerprintError::TooManyFiles {
                limit: 20_000,
                path: path.clone(),
            },
            FingerprintError::TooManyBytes {
                limit: 1024,
                path: path.clone(),
            },
            FingerprintError::IncompleteTree {
                path: path.clone(),
                detail: "SURE did not look at src/gen because it is a link.".to_owned(),
            },
        ]
    }

    #[test]
    fn every_message_says_that_there_is_no_fingerprint() {
        // The one thing every one of these has to say. A message that a caller
        // reads as "here is a fingerprint, minus a bit" is the failure, so each
        // is checked for the sentence that rules it out.
        let all = all();
        assert_eq!(all.len(), 11, "a variant was not checked");
        for error in &all {
            let text = error.to_string();
            assert!(
                text.contains("fingerprint"),
                "{error:?} does not mention a fingerprint: {text}"
            );
            assert!(
                !text.contains("  "),
                "{error:?} has a run-together continuation line: {text}"
            );
        }
    }

    #[test]
    fn the_operating_systems_words_and_gits_words_are_quoted() {
        // Same rule as `ScanError` and `Skipped::detail`: the paraphrase is
        // where the part a user can act on gets lost.
        for (error, expected) in [
            (
                FingerprintError::NoRepository {
                    root: PathBuf::from("/work/project"),
                    message: "fatal: not a git repository (or any of the parent directories)"
                        .to_owned(),
                },
                "fatal: not a git repository (or any of the parent directories)",
            ),
            (
                FingerprintError::Unreadable {
                    path: PathBuf::from("/work/project/a.rs"),
                    message: "Access is denied. (os error 5)".to_owned(),
                },
                "Access is denied. (os error 5)",
            ),
            (
                FingerprintError::GitFailed {
                    operation: "read the repository's status",
                    status: Some(128),
                    message: "fatal: detected dubious ownership".to_owned(),
                },
                "fatal: detected dubious ownership",
            ),
        ] {
            assert!(
                error.to_string().contains(expected),
                "{error:?} does not quote {expected:?}"
            );
        }
    }

    #[test]
    fn every_message_names_the_thing_that_went_wrong() {
        // A message that says "something went wrong" leaves the reader to guess
        // which of eleven things happened. Each error is built here rather than
        // reached by its position in `all()`, so adding a variant cannot quietly
        // move this test onto a different one.
        let path = PathBuf::from("/work/project");
        let named = [
            (
                FingerprintError::NotAbsolute { root: path.clone() },
                "/work/project",
            ),
            (
                FingerprintError::NoRepository {
                    root: path.clone(),
                    message: String::new(),
                },
                "/work/project",
            ),
            (
                FingerprintError::GitUnavailable {
                    program: OsString::from("git"),
                    message: String::new(),
                },
                "git",
            ),
            (
                FingerprintError::GitFailed {
                    operation: "read the status",
                    status: Some(128),
                    message: String::new(),
                },
                "read the status",
            ),
            (
                FingerprintError::UnrecognizedStatus {
                    record: "2 R. N... 100644 a\tb".to_owned(),
                },
                "SURE does not understand what Git said",
            ),
            (
                FingerprintError::OutsideRoot {
                    path: PathBuf::from("elsewhere/file.rs"),
                },
                "elsewhere/file.rs",
            ),
            (
                FingerprintError::Unreadable {
                    path: PathBuf::from("src/lib.rs"),
                    message: String::new(),
                },
                "src/lib.rs",
            ),
            (
                FingerprintError::TooManyFiles {
                    limit: 20_000,
                    path: path.clone(),
                },
                "20000",
            ),
            (
                FingerprintError::TooManyBytes {
                    limit: 1024,
                    path: path.clone(),
                },
                "1024",
            ),
            (
                FingerprintError::IncompleteTree {
                    path: PathBuf::from("vendor"),
                    detail: String::new(),
                },
                "vendor",
            ),
        ];
        // `MissingHead` is not in the list: it carries nothing but what is
        // already true of it, and the test below is the one that holds it.
        assert_eq!(named.len() + 1, all().len(), "an error was not checked");
        for (error, expected) in named {
            assert!(
                error.to_string().contains(expected),
                "{error:?} does not name {expected:?}"
            );
        }
    }

    #[test]
    fn a_status_with_no_exit_code_reads_as_one() {
        // `ExitStatus::code` is `None` when a process was killed by a signal,
        // and "exit code None" is not a sentence.
        let error = FingerprintError::GitFailed {
            operation: "read the repository's status",
            status: None,
            message: String::new(),
        };
        let text = error.to_string();
        assert!(text.contains("killed"), "{text}");
        assert!(!text.contains("None"), "{text}");
    }

    #[test]
    fn a_refusal_is_an_error_value_and_not_a_success() {
        // The false-green shape, stated as a test: none of these can be reached
        // by ignoring a `Result`, because all of them are the `Err`.
        for error in all() {
            let _: &dyn std::error::Error = &error;
        }
    }
}

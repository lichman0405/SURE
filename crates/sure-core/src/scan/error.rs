//! What stops a scan before it starts.
//!
//! Everything that goes wrong *inside* a walk is a [`Skipped`] value, not an
//! error, because a scan that found most of a project and could not read one
//! directory is a useful scan and the loss belongs in its result. These four
//! are different: each means there is no scan at all, and returning an empty
//! [`Scan`] instead would be the worst possible answer, because an empty scan
//! and a project with nothing in it look the same.
//!
//! [`Skipped`]: super::Skipped
//! [`Scan`]: super::Scan

use std::fmt;
use std::path::PathBuf;

/// Why a scan could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    /// The root was given as a relative path.
    ///
    /// Resolving it against the current directory would make what SURE scanned
    /// depend on where it happened to be started, and every path in the result
    /// would be relative to a directory nobody named.
    NotAbsolute {
        /// The root as it was given.
        root: PathBuf,
    },
    /// There is nothing at the root.
    Missing {
        /// The root as it was given.
        root: PathBuf,
    },
    /// There is something at the root, and it is not a directory.
    NotADirectory {
        /// The root as it was given.
        root: PathBuf,
    },
    /// The root is a directory SURE could not list.
    Unreadable {
        /// The root as it was given.
        root: PathBuf,
        /// What the operating system said.
        message: String,
    },
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute { root } => write!(
                f,
                "SURE was asked to look at \"{}\", which is not a full path.\n\n\
                 A short path would be resolved against whatever directory SURE happened to be \
                 started in, so the same command would read a different project depending on \
                 where it ran — and everything the scan reported would be relative to a \
                 directory nobody named.\n\n\
                 Give the whole path, or run SURE from the project's own directory and name it \
                 as \".\".",
                root.display()
            ),
            Self::Missing { root } => write!(
                f,
                "There is nothing at \"{}\".\n\n\
                 SURE stopped rather than report on an empty project, because a project with no \
                 files in it and a path that does not exist would then look the same.\n\n\
                 Check the path, or change to the project's directory first.",
                root.display()
            ),
            Self::NotADirectory { root } => write!(
                f,
                "\"{}\" is a file, not a project directory.\n\n\
                 SURE stops rather than read the file and call the result a project.\n\n\
                 Give the folder that contains it.",
                root.display()
            ),
            Self::Unreadable { root, message } => write!(
                f,
                "SURE could not look inside \"{}\".\n\n\
                 The operating system said: {message}\n\n\
                 Nothing about this project was read, so there is nothing to report. Check that \
                 the folder exists and that you have permission to open it.",
                root.display()
            ),
        }
    }
}

impl std::error::Error for ScanError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn all() -> Vec<ScanError> {
        let root = PathBuf::from("/work/project");
        vec![
            ScanError::NotAbsolute { root: root.clone() },
            ScanError::Missing { root: root.clone() },
            ScanError::NotADirectory { root: root.clone() },
            ScanError::Unreadable {
                root,
                message: "Access is denied.".to_owned(),
            },
        ]
    }

    #[test]
    fn every_message_names_the_path_and_says_what_sure_did_instead() {
        // One expected phrase per variant, written out rather than a loose
        // "contains one of these". A message that says the path was wrong but
        // not what SURE did about it leaves the reader to guess whether a scan
        // happened, and the guess that matters — "it must have scanned what it
        // could" — is the wrong one.
        let expected = [
            (
                ScanError::NotAbsolute {
                    root: PathBuf::from("/work/project"),
                },
                "which is not a full path",
            ),
            (
                ScanError::Missing {
                    root: PathBuf::from("/work/project"),
                },
                "There is nothing at",
            ),
            (
                ScanError::NotADirectory {
                    root: PathBuf::from("/work/project"),
                },
                "is a file, not a project directory",
            ),
            (
                ScanError::Unreadable {
                    root: PathBuf::from("/work/project"),
                    message: "Access is denied.".to_owned(),
                },
                "Nothing about this project was read",
            ),
        ];

        assert_eq!(expected.len(), all().len(), "a variant was not checked");
        for (error, phrase) in &expected {
            let text = error.to_string();
            assert!(text.contains("/work/project"), "{text}");
            assert!(
                text.contains(phrase),
                "{error:?} does not say what SURE did instead: expected {phrase:?} in {text}"
            );
        }
    }

    #[test]
    fn the_operating_systems_words_are_quoted_rather_than_paraphrased() {
        // Same reason as `Skipped::detail`: the paraphrase is where the part a
        // user can act on gets lost.
        let error = ScanError::Unreadable {
            root: PathBuf::from("/work/project"),
            message: "Access is denied. (os error 5)".to_owned(),
        };
        assert!(error.to_string().contains("Access is denied. (os error 5)"));
    }

    #[test]
    fn a_refusal_is_not_a_success() {
        // The false-green shape, stated as a test: every one of these is an
        // error value, so no caller can reach a `Scan` by ignoring one.
        for error in all() {
            assert!(!error.to_string().is_empty());
            let _: &dyn std::error::Error = &error;
        }
    }
}

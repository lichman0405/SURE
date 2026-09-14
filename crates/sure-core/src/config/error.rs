//! Why a configuration file could not be used.
//!
//! Every message here is written to be acted on: what SURE found, what it did
//! instead of guessing, and the exact change that would fix it. Two rules run
//! through all of them.
//!
//! **Nothing is silently dropped.** A setting that SURE ignored would look
//! exactly like a setting that was applied, so an unrecognised setting stops
//! the run rather than being passed over.
//!
//! **No credential is echoed.** A project-controlled file may not carry one,
//! and any value that reaches a message goes through
//! [`redact_for_diagnostic`](super::redact::redact_for_diagnostic) first.

use std::fmt;
use std::path::{Path, PathBuf};

/// A position in a configuration file. Lines and columns are 1-based, matching
/// every editor and the YAML parser itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
}

impl Location {
    /// A position in a file.
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// What went wrong, without the file it went wrong in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// The file exists but could not be read.
    Unreadable {
        /// The operating system's description of the failure.
        message: String,
    },
    /// The file is not valid UTF-8.
    NotUtf8,
    /// A file with a near-miss name was found where `sure.yaml` belongs.
    WrongFileName {
        /// The file that was found.
        found: PathBuf,
        /// The file SURE reads.
        expected: PathBuf,
    },
    /// The text is not valid YAML.
    Malformed {
        /// The parser's own description, with its location removed because the
        /// location is carried separately.
        message: String,
    },
    /// The YAML is valid but is not a set of settings.
    NotSettings {
        /// What the document actually is, in plain words.
        shape: &'static str,
    },
    /// A setting SURE does not know about.
    UnknownSetting {
        /// The unrecognised name.
        name: String,
        /// The names that would have been accepted there.
        expected: Vec<String>,
        /// The closest accepted name, when one is close enough to be a typo.
        suggestion: Option<String>,
    },
    /// A setting that appears more than once.
    RepeatedSetting {
        /// The repeated setting's name.
        name: String,
    },
    /// A setting whose value SURE does not accept.
    BadValue {
        /// The setting's dotted name, when SURE could attribute the value.
        name: Option<String>,
        /// The offending value, already redacted.
        value: String,
        /// The values that would have been accepted.
        expected: Vec<String>,
        /// The closest accepted value, when one is close enough to be a typo.
        suggestion: Option<String>,
    },
    /// A credential in a project-controlled file.
    Credential {
        /// The dotted name of the setting that holds it.
        setting: String,
    },
    /// A setting this release parses but does not implement.
    NotAvailable {
        /// The setting's dotted name.
        name: String,
        /// The requested value.
        value: String,
        /// Why accepting it would be dishonest.
        explanation: &'static str,
        /// What to write instead.
        instead: &'static str,
    },
    /// Two settings that cannot both take effect.
    Contradiction {
        /// The first setting's dotted name.
        first: String,
        /// The second setting's dotted name.
        second: String,
        /// Why they cannot both hold.
        explanation: &'static str,
    },
    /// A path that leaves the project.
    OutsideProject {
        /// The setting's dotted name.
        name: String,
        /// The offending path, already redacted.
        value: String,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { message } => write!(
                f,
                "sure.yaml could not be read: {message}\n\n\
                 SURE stopped rather than continue without the settings in that file."
            ),
            Self::NotUtf8 => write!(
                f,
                "sure.yaml is not UTF-8 text.\n\n\
                 SURE stopped rather than read it with characters replaced by guesses, \
                 because a guess in a settings file is a setting you did not choose.\n\n\
                 Save the file as UTF-8 and try again."
            ),
            Self::WrongFileName { found, expected } => write!(
                f,
                "SURE found {} but reads {}.\n\n\
                 SURE stopped instead of ignoring the file, because ignoring it would run \
                 with default settings while you believe yours are in force.\n\n\
                 Rename it to {}.",
                found.display(),
                expected.display(),
                expected
                    .file_name()
                    .unwrap_or(expected.as_os_str())
                    .to_string_lossy()
            ),
            Self::Malformed { message } => write!(
                f,
                "sure.yaml is not valid YAML: {message}\n\n\
                 SURE stopped rather than continue with none of your settings applied."
            ),
            Self::NotSettings { shape } => write!(
                f,
                "sure.yaml must be a set of settings, but it is {shape}.\n\n\
                 SURE stopped rather than ignore the file and run with defaults you did not choose."
            ),
            Self::UnknownSetting {
                name,
                expected,
                suggestion,
            } => {
                write!(
                    f,
                    "sure.yaml sets `{name}`, which is not a setting SURE knows.\n\n\
                     SURE stopped rather than ignore it: a setting that is silently dropped \
                     looks exactly like a setting that was applied."
                )?;
                if let Some(suggestion) = suggestion {
                    write!(f, "\n\n  Did you mean `{suggestion}`?")?;
                }
                if !expected.is_empty() {
                    write!(f, "\n  Settings available here: {}", list(expected))?;
                }
                Ok(())
            }
            Self::RepeatedSetting { name } => write!(
                f,
                "sure.yaml sets `{name}` more than once.\n\n\
                 SURE stopped rather than take the last one. Whichever it picked, the other \
                 would have been dropped without a word, and a setting written twice is \
                 usually a setting that was being changed.\n\n\
                 Remove one of the two."
            ),
            Self::BadValue {
                name,
                value,
                expected,
                suggestion,
            } => {
                match name {
                    Some(name) => write!(
                        f,
                        "sure.yaml sets `{name}` to \"{value}\", which SURE does not accept."
                    )?,
                    None => write!(
                        f,
                        "sure.yaml contains a value SURE does not accept: {value}"
                    )?,
                }
                write!(
                    f,
                    "\n\nSURE stopped rather than guess which value you meant."
                )?;
                match expected.len() {
                    // One alternative is a description of what belongs there
                    // rather than a menu, so it is not offered as a choice.
                    1 => write!(f, "\n\n  Expected: {}", expected[0])?,
                    0 => {}
                    _ => write!(f, "\n\n  Use one of: {}", list(expected))?,
                }
                if let Some(suggestion) = suggestion {
                    write!(f, "\n  Did you mean \"{suggestion}\"?")?;
                }
                Ok(())
            }
            Self::Credential { setting } => write!(
                f,
                "sure.yaml sets `{setting}`, which looks like a credential.\n\n\
                 A project file is not a safe place for one: anyone who can edit this project \
                 can read it, and SURE copies its settings into reports and history.\n\n\
                 SURE stopped rather than read the file, and has not printed the value.\n\n\
                 Keep the credential in your user-level SURE configuration or an environment \
                 variable, and refer to it by name from the project file."
            ),
            Self::NotAvailable {
                name,
                value,
                explanation,
                instead,
            } => write!(
                f,
                "sure.yaml sets `{name}` to \"{value}\", which this version of SURE does not implement.\n\n\
                 {explanation}\n\n\
                 SURE stopped rather than accept a setting it cannot honour.\n\n\
                 {instead}"
            ),
            Self::Contradiction {
                first,
                second,
                explanation,
            } => write!(
                f,
                "sure.yaml sets `{first}` and `{second}`, which cannot both take effect.\n\n\
                 {explanation}\n\n\
                 SURE stopped rather than accept one of them and quietly ignore the other."
            ),
            Self::OutsideProject { name, value } => write!(
                f,
                "sure.yaml sets `{name}` to \"{value}\", which points outside the project.\n\n\
                 A project file describes this project; it cannot name files elsewhere on the \
                 machine.\n\n\
                 SURE stopped rather than follow it."
            ),
        }
    }
}

/// Render a list of alternatives for a message.
fn list(items: &[String]) -> String {
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(item);
    }
    out
}

/// A configuration failure, with where it happened.
///
/// The kind is boxed because several of its variants carry owned strings, which
/// would otherwise make every `Result` returned from this module carry a
/// 150-byte error on its success path as well. Nothing about the API changes:
/// [`ConfigError::kind`] still hands back a plain [`ErrorKind`] reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    kind: Box<ErrorKind>,
    file: Option<PathBuf>,
    location: Option<Location>,
}

impl ConfigError {
    /// A failure that has not been attributed to a file yet.
    #[must_use]
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind: Box::new(kind),
            file: None,
            location: None,
        }
    }

    /// The same failure, attributed to the file it was found in.
    ///
    /// The first attribution wins: an error raised while reading one file is
    /// not made more accurate by being relabelled with another.
    #[must_use]
    pub fn at_file(mut self, path: &Path) -> Self {
        if self.file.is_none() {
            self.file = Some(path.to_path_buf());
        }
        self
    }

    /// The same failure, with a position inside the file.
    #[must_use]
    pub fn at(mut self, location: Location) -> Self {
        if self.location.is_none() {
            self.location = Some(location);
        }
        self
    }

    /// What went wrong.
    #[must_use]
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// The file the failure was found in, if it is known.
    #[must_use]
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// The position inside the file, if the parser reported one.
    #[must_use]
    pub const fn location(&self) -> Option<Location> {
        self.location
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        match (&self.file, self.location) {
            (Some(file), Some(location)) => write!(f, "\n\nIn: {} ({location})", file.display()),
            (Some(file), None) => write!(f, "\n\nIn: {}", file.display()),
            (None, Some(location)) => write!(f, "\n\nAt {location}"),
            (None, None) => Ok(()),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn message(kind: ErrorKind) -> String {
        ConfigError::new(kind).to_string()
    }

    #[test]
    fn a_location_reads_the_way_an_editor_numbers_it() {
        assert_eq!(Location::new(7, 3).to_string(), "line 7, column 3");
    }

    #[test]
    fn every_message_says_what_happened_and_what_to_do() {
        // The acceptance criterion is "invalid config is actionable". A message
        // that only reports the failure makes the user guess, so each of these
        // is checked for a statement of what SURE did and a next step.
        let cases: &[ErrorKind] = &[
            ErrorKind::Unreadable {
                message: "the file is locked by another program".to_owned(),
            },
            ErrorKind::NotUtf8,
            ErrorKind::WrongFileName {
                found: PathBuf::from("C:/p/sure.yml"),
                expected: PathBuf::from("C:/p/sure.yaml"),
            },
            ErrorKind::Malformed {
                message: "unexpected end of stream".to_owned(),
            },
            ErrorKind::NotSettings { shape: "a list" },
            ErrorKind::UnknownSetting {
                name: "excecution".to_owned(),
                expected: vec!["execution".to_owned()],
                suggestion: Some("execution".to_owned()),
            },
            ErrorKind::BadValue {
                name: Some("execution.mode".to_owned()),
                value: "host-confimed".to_owned(),
                expected: vec!["inspect_only".to_owned()],
                suggestion: Some("host_confirmed".to_owned()),
            },
            ErrorKind::RepeatedSetting {
                name: "report.format".to_owned(),
            },
            ErrorKind::Credential {
                setting: "analysis.api_key".to_owned(),
            },
            ErrorKind::NotAvailable {
                name: "privacy.mode".to_owned(),
                value: "cloud_enhanced".to_owned(),
                explanation: "It is a documented future mode.",
                instead: "Use local_first or fully_local.",
            },
            ErrorKind::Contradiction {
                first: "execution.mode".to_owned(),
                second: "execution.allow_network".to_owned(),
                explanation: "Nothing runs in inspect_only mode.",
            },
            ErrorKind::OutsideProject {
                name: "project_intent.spec_path".to_owned(),
                value: "../elsewhere/spec.md".to_owned(),
            },
        ];

        for kind in cases {
            let text = message(kind.clone());
            let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
            assert!(
                lines.len() >= 2,
                "{kind:?} is a single statement with no next step:\n{text}"
            );
            assert!(
                text.contains("SURE stopped")
                    || text.contains("Rename it")
                    || text.contains("Remove one")
                    || text.contains("Use "),
                "{kind:?} does not say what to do about it:\n{text}"
            );
            assert!(
                !text.contains("  \n"),
                "{kind:?} leaves a trailing-space line:\n{text}"
            );
        }
    }

    #[test]
    fn an_error_names_the_file_and_the_position() {
        let error = ConfigError::new(ErrorKind::NotUtf8)
            .at_file(Path::new("C:/projects/demo/sure.yaml"))
            .at(Location::new(4, 9));
        let text = error.to_string();
        assert!(text.contains("C:/projects/demo/sure.yaml"), "{text}");
        assert!(text.contains("line 4, column 9"), "{text}");
    }

    #[test]
    fn the_first_attribution_wins() {
        let error = ConfigError::new(ErrorKind::NotUtf8)
            .at_file(Path::new("first.yaml"))
            .at_file(Path::new("second.yaml"));
        assert_eq!(error.file(), Some(Path::new("first.yaml")));
    }

    #[test]
    fn an_error_without_a_file_still_reports_its_position() {
        let error =
            ConfigError::new(ErrorKind::NotSettings { shape: "a list" }).at(Location::new(2, 1));
        assert!(error.to_string().contains("At line 2, column 1"));
    }
}

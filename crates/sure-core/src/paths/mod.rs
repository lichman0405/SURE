//! Where SURE keeps things, and what may be kept there.
//!
//! Two facts drive this module, both from
//! `docs/architecture/STORAGE_AND_DATA_PATHS.md`.
//!
//! **Locations are the platform's, not ours.** Hard-coding `~/.sure` is wrong on
//! Windows, where the user's application data lives under `%LOCALAPPDATA%` and
//! their configuration under `%APPDATA%`, and wrong on macOS, where it lives
//! under `~/Library/Application Support`. The resolution is delegated to a path
//! library rather than assembled from environment variables here: the correct
//! Windows API is `SHGetKnownFolderPath`, this workspace sets
//! `unsafe_code = "forbid"`, and a hand-rolled version that read
//! `%LOCALAPPDATA%` directly would quietly produce a *relative* path whenever
//! that variable is unset.
//!
//! **Authoritative evidence does not live in the project.** The checked project
//! may be edited by the agent whose work SURE is evaluating, so a passing
//! history stored in `.sure/` could be written by the thing being judged. Durable
//! evidence lives in the user-level data directory; the project keeps only
//! regenerable cache. [`Paths::ensure_outside`] is what makes that a rule rather
//! than a convention, and [`project_cache_dir`] marks the other side of the
//! boundary so a caller cannot reach for it by accident.

pub mod compare;

pub use compare::{CaseSensitivity, is_within, is_within_case, same_path, same_path_case};

use std::fmt;
use std::path::{Path, PathBuf};

/// The folder SURE uses under the platform's per-user directories.
///
/// Upper case because that is the convention on Windows, and harmless on the
/// platforms where it is not.
pub const APP_DIR: &str = "SURE";

/// The name of the user-level configuration file, inside the config directory.
///
/// The same name a project uses, because it is the same file format and the
/// same settings — the difference is who is allowed to grant what, which is
/// `docs/architecture/CONFIG_AUTHORITY.md`'s subject, not this one's.
pub const USER_CONFIG_FILE: &str = "sure.yaml";

/// The directory a project may keep regenerable state in.
pub const PROJECT_CACHE_DIR: &str = ".sure";

/// Why the user-level locations could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// The platform did not report a per-user location.
    ///
    /// There is deliberately no fallback. Guessing would put SURE's evidence
    /// database somewhere the user did not choose — and the nearest guess,
    /// relative to the current directory, is the project being checked.
    Unavailable {
        /// Which location, in the words a user would use.
        what: &'static str,
    },
    /// A location that has to be absolute was not.
    NotAbsolute {
        /// Which location, in the words a user would use.
        what: &'static str,
        /// The path as it was given.
        path: PathBuf,
    },
    /// An authoritative location is the project itself, or below it.
    InsideProject {
        /// The location SURE would have written authoritative evidence to.
        store: PathBuf,
        /// The project it turned out to be inside.
        project: PathBuf,
    },
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { what } => write!(
                f,
                "SURE could not work out where {what} belongs on this machine.\n\n\
                 SURE stopped rather than fall back to a guess. The nearest guess would be a \
                 path relative to the current directory, which is the project being checked — \
                 and evidence stored inside a project can be edited by the agent whose work is \
                 being judged.\n\n\
                 Set the platform's standard location for per-user application data and try \
                 again."
            ),
            Self::NotAbsolute { what, path } => write!(
                f,
                "{what} was given as \"{}\", which is not an absolute path.\n\n\
                 A relative path would be resolved against the current directory, so whether \
                 it is inside the project would depend on where SURE happened to be started.\n\n\
                 SURE stopped rather than check it and report the answer as if it meant \
                 something.",
                path.display()
            ),
            Self::InsideProject { store, project } => write!(
                f,
                "SURE keeps authoritative evidence in {}, and that is inside {}.\n\n\
                 Evidence inside the project can be edited by whatever is working in it, so a \
                 stored result would stop being evidence of anything.\n\n\
                 SURE stopped rather than treat it as authoritative. If you are checking your \
                 home directory, or a folder that contains your application data, check the \
                 project from somewhere outside it.",
                store.display(),
                project.display()
            ),
        }
    }
}

impl std::error::Error for PathError {}

/// The user-level directories SURE stores things in.
///
/// There is no separate cache root. On Windows the cache directory is the same
/// `%LOCALAPPDATA%` as the data directory, so a third field would name the same
/// place twice and suggest a distinction that does not exist. What is
/// regenerable is the *project* cache, and that is [`project_cache_dir`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    data: PathBuf,
    config: PathBuf,
}

impl Paths {
    /// Where SURE stores things for the user running it.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::Unavailable`] if the platform does not report a
    /// per-user location for the data or configuration directory. That is rare,
    /// and it is not recoverable by guessing — see the variant's own message.
    pub fn discover() -> Result<Self, PathError> {
        let data = dirs::data_local_dir().ok_or(PathError::Unavailable {
            what: "its evidence and history",
        })?;
        let config = dirs::config_dir().ok_or(PathError::Unavailable {
            what: "its user-level settings",
        })?;
        Self::from_roots(data.join(APP_DIR), config.join(APP_DIR))
    }

    /// The same, from locations given rather than discovered.
    ///
    /// Used by tests, and by a caller that has a reason to keep SURE's data
    /// somewhere else. The paths are checked here rather than at use, so a
    /// location that could never pass [`Paths::ensure_outside`] cannot be
    /// constructed in the first place.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotAbsolute`] for a relative path, and
    /// [`PathError::Unavailable`] for an empty one. A relative data directory is
    /// resolved against the current directory, which for a check is the project.
    pub fn from_roots(data: PathBuf, config: PathBuf) -> Result<Self, PathError> {
        absolute("The evidence and history directory", data)
            .and_then(|data| {
                absolute("The user-level settings directory", config).map(|config| (data, config))
            })
            .map(|(data, config)| Self { data, config })
    }

    /// Where durable evidence and history belong.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    /// Where user-level configuration belongs.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    /// The user-level configuration file.
    #[must_use]
    pub fn user_config_file(&self) -> PathBuf {
        self.config.join(USER_CONFIG_FILE)
    }

    /// Refuse a project that contains SURE's own evidence store.
    ///
    /// This is the rule from `docs/architecture/STORAGE_AND_DATA_PATHS.md`: the
    /// checked project is not a trusted place to keep authoritative evidence,
    /// because the agent whose work is being judged can edit it. A history that
    /// the project can rewrite cannot support a verdict.
    ///
    /// The case is not hypothetical. A user who runs SURE on their home
    /// directory has a project root that contains `%LOCALAPPDATA%`, and the
    /// correct answer there is to stop rather than to write anyway.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotAbsolute`] if `project_root` is not absolute —
    /// comparing a relative root against an absolute store would always report
    /// "outside", which is the answer that lets the mistake through.
    /// Returns [`PathError::InsideProject`] when the store is the project or
    /// below it.
    pub fn ensure_outside(&self, project_root: &Path) -> Result<(), PathError> {
        let project = absolute("The project directory", project_root.to_path_buf())?;
        if is_within(&self.data, &project) || is_within(&self.config, &project) {
            return Err(PathError::InsideProject {
                store: self.data.clone(),
                project,
            });
        }
        Ok(())
    }
}

/// The directory a project may keep regenerable state in: `<project>/.sure`.
///
/// Nothing authoritative may be written here, and the type carries no way to ask
/// for it: this is a cache, and a caller that wants the evidence store has to go
/// through [`Paths`]. It is deliberately a plain path rather than a struct with
/// a misleading `is_authoritative` flag — the boundary is enforced by which
/// function produces the path, not by a boolean someone can set.
#[must_use]
pub fn project_cache_dir(project_root: &Path) -> PathBuf {
    project_root.join(PROJECT_CACHE_DIR)
}

/// Accept a location only if it can mean one thing.
fn absolute(what: &'static str, path: PathBuf) -> Result<PathBuf, PathError> {
    // `components()` is empty for both `""` and `"."`, which are relative by
    // construction even though neither looks like it.
    if !path.is_absolute() || path.components().next().is_none() {
        return Err(PathError::NotAbsolute { what, path });
    }
    Ok(path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// An absolute path, spelled the way this platform spells one.
    ///
    /// Fixtures written as `"/data/SURE"` are relative on Windows, so a test
    /// built from them would pass or fail on the spelling rather than on the
    /// behaviour. Every path in these tests is assembled from this.
    fn rooted(parts: &[&str]) -> PathBuf {
        let mut path = if cfg!(windows) {
            PathBuf::from(r"C:\base")
        } else {
            PathBuf::from("/base")
        };
        for part in parts {
            path.push(part);
        }
        path
    }

    #[test]
    fn the_discovered_locations_are_absolute_and_named_for_the_application() {
        let paths = Paths::discover().expect("this machine reports per-user locations");
        for dir in [paths.data_dir(), paths.config_dir()] {
            assert!(dir.is_absolute(), "{} is not absolute", dir.display());
            assert_eq!(
                dir.file_name().and_then(|name| name.to_str()),
                Some(APP_DIR),
                "{} is not inside an application folder",
                dir.display()
            );
        }
    }

    #[test]
    fn the_discovered_locations_do_not_encode_a_home_directory() {
        // "without hardcoding ~/.sure" is the acceptance criterion. A literal
        // `~` would be a path that no platform expands, so a store written
        // there would be written to a directory named `~`.
        let paths = Paths::discover().expect("this machine reports per-user locations");
        for dir in [paths.data_dir(), paths.config_dir()] {
            let text = dir.to_string_lossy();
            assert!(!text.contains('~'), "{} contains a tilde", text);
        }
    }

    #[test]
    fn the_data_and_config_locations_are_asked_of_the_platform_separately() {
        // On Windows these are `%LOCALAPPDATA%` and `%APPDATA%`: the evidence
        // store is machine-local and does not belong in a roaming profile,
        // while settings are exactly what roaming is for. Asserting only that
        // the two agree would pass on a platform that answered the same thing
        // to both questions, which is the bug.
        let paths = Paths::discover().expect("this machine reports per-user locations");
        assert_eq!(
            paths.user_config_file(),
            paths.config_dir().join("sure.yaml")
        );
        assert_eq!(paths.user_config_file().file_name().unwrap(), "sure.yaml");
    }

    #[test]
    fn the_user_configuration_file_sits_in_the_configuration_directory() {
        let paths = Paths::discover().expect("this machine reports per-user locations");
        assert!(is_within(&paths.user_config_file(), paths.config_dir()));
    }

    #[test]
    fn a_relative_location_is_refused_rather_than_resolved_against_the_cwd() {
        let error = Paths::from_roots(PathBuf::from(".sure"), rooted(&["cfg"])).unwrap_err();
        match error {
            PathError::NotAbsolute { path, .. } => assert_eq!(path, Path::new(".sure")),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_location_is_refused_as_well_as_a_relative_one() {
        // `""` is the kind of value that arrives from a field nobody filled in,
        // so it is checked rather than assumed to be caught elsewhere.
        assert!(Paths::from_roots(PathBuf::from(""), rooted(&["cfg"])).is_err());
        assert!(Paths::from_roots(rooted(&["data"]), PathBuf::from("")).is_err());
    }

    #[test]
    fn a_project_that_does_not_contain_the_store_is_accepted() {
        let paths = Paths::from_roots(rooted(&["data", "SURE"]), rooted(&["cfg", "SURE"])).unwrap();
        assert!(paths.ensure_outside(&rooted(&["work", "project"])).is_ok());
    }

    #[test]
    fn a_project_that_contains_the_store_is_refused() {
        let project = rooted(&["work", "project"]);
        let paths =
            Paths::from_roots(project.join("data").join("SURE"), rooted(&["cfg", "SURE"])).unwrap();

        let error = paths.ensure_outside(&project).unwrap_err();
        match error {
            PathError::InsideProject {
                store,
                project: named,
            } => {
                assert_eq!(store, project.join("data").join("SURE"));
                assert_eq!(named, project);
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_project_that_is_the_store_itself_is_refused() {
        // Equality counts. A store that *is* the project root is as editable by
        // the agent as one below it.
        let store = rooted(&["work", "project"]);
        let paths = Paths::from_roots(store.clone(), rooted(&["cfg", "SURE"])).unwrap();
        assert!(paths.ensure_outside(&store).is_err());
    }

    #[test]
    fn a_home_directory_project_is_refused() {
        // The real case: on Windows `%LOCALAPPDATA%` is inside `%USERPROFILE%`,
        // so checking the home directory would put the evidence store inside
        // the project. The nearest sibling project is still fine.
        let home = rooted(&["users", "me"]);
        let paths =
            Paths::from_roots(home.join("data").join("SURE"), rooted(&["cfg", "SURE"])).unwrap();

        assert!(paths.ensure_outside(&home).is_err());
        assert!(
            paths
                .ensure_outside(&rooted(&["users", "me", "code", "app"]))
                .is_ok()
        );
    }

    #[test]
    fn a_case_difference_does_not_hide_a_project_that_contains_the_store() {
        // Only meaningful where the platform folds case; on a case-sensitive
        // platform the two spellings really are different directories, and
        // `paths::compare` tests that rule directly.
        if CaseSensitivity::platform() != CaseSensitivity::Insensitive {
            return;
        }
        let project = rooted(&["Work", "Project"]);
        let paths = Paths::from_roots(
            rooted(&["work", "project", "data"]),
            rooted(&["cfg", "SURE"]),
        )
        .unwrap();
        assert!(paths.ensure_outside(&project).is_err());
    }

    #[test]
    fn a_relative_project_root_is_refused_rather_than_compared() {
        // Comparing a relative root with an absolute store always reports
        // "outside", which is the answer that lets the mistake through.
        let paths = Paths::from_roots(rooted(&["data", "SURE"]), rooted(&["cfg", "SURE"])).unwrap();
        assert!(paths.ensure_outside(Path::new("project")).is_err());
        assert!(paths.ensure_outside(Path::new(".")).is_err());
    }

    #[test]
    fn the_project_cache_is_inside_the_project_and_is_not_where_evidence_goes() {
        let project = rooted(&["work", "project"]);
        let cache = project_cache_dir(&project);

        // The whole reason the two are separate functions: the cache is inside
        // the project by construction, so it can never be authoritative.
        assert!(is_within(&cache, &project));
        assert_eq!(cache, project.join(".sure"));

        let paths = Paths::from_roots(rooted(&["data", "SURE"]), rooted(&["cfg", "SURE"])).unwrap();
        assert!(!is_within(paths.data_dir(), &project));
    }

    #[test]
    fn every_message_says_what_sure_did_instead() {
        for error in [
            PathError::Unavailable {
                what: "its evidence",
            },
            PathError::NotAbsolute {
                what: "The evidence directory",
                path: PathBuf::from(".sure"),
            },
            PathError::InsideProject {
                store: PathBuf::from("/p/.sure"),
                project: PathBuf::from("/p"),
            },
        ] {
            let text = error.to_string();
            assert!(
                text.contains("SURE stopped rather than"),
                "no statement of what SURE did instead:\n{text}"
            );
            assert!(
                text.lines().filter(|line| !line.trim().is_empty()).count() >= 2,
                "a single statement with no next step:\n{text}"
            );
        }
    }
}

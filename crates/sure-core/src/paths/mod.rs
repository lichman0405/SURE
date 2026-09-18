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
//!
//! **The store's location can be named by the caller, and by nothing else.** A
//! run may keep its store somewhere the person running SURE chose, which is what
//! [`Paths::discover_at`] and `sure --store-dir` are for, and that value comes
//! from exactly one place: the process's own argument vector. Nothing is read
//! from the project — no `.sure/config`, no field in a manifest, no file beside
//! the sources — because a location a checked project's own file could name is a
//! location a checked project could point at a directory it can write to, and
//! then the history a verdict is read from would be the history the judged thing
//! writes. There is deliberately no environment variable either: a checked
//! project's harness configuration can set the environment of the processes it
//! starts, so a variable would be the same hole with a different name. The
//! default is not a fallback chain — it is the platform's own per-user location
//! through [`Paths::discover`], unchanged, and a caller who names nothing gets
//! it. Tests in `crates/sure-cli/tests/cli_contract.rs` keep this true rather
//! than merely stated: `nothing_a_project_can_write_decides_where_the_store_goes`
//! scans the modules that decide the location for the shape that would break it
//! (a read of the environment), `every_command_is_reached_by_the_location_the_caller_named`
//! scans every crate's shipped code for a call to the no-argument
//! [`Paths::discover`] that would ignore what the caller named,
//! `a_named_store_directory_is_the_one_a_real_run_writes_to` and
//! `a_doctor_report_says_which_store_location_the_run_is_using` drive a real
//! binary to a named location and to the default one respectively, and
//! `a_store_inside_the_project_is_refused_before_anything_is_recorded` holds the
//! refusal to [`Paths::ensure_outside`].

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

/// The name of the local record store, inside the data directory.
///
/// The name is here rather than in `crate::store` so that every fact about
/// *where* SURE puts things stays in one module, and so that a caller cannot
/// assemble a path to the store by hand — [`Paths::store_file`] is the only way
/// to name it.
pub const STORE_FILE: &str = "sure.db";

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

/// Where a run's store location came from.
///
/// The two answers are the two ways a location comes to exist, and they are what
/// `sure doctor` prints so that a caller can tell a requested location from the
/// platform's own: a redirect that was ignored and a redirect that worked look
/// identical in a path, and the reader would debug the wrong thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The platform's own per-user location, resolved through `dirs`.
    ///
    /// The default, and the only thing a caller who names nothing gets.
    Platform,
    /// A directory a caller named rather than discovered.
    ///
    /// Named by [`Paths::discover_at`]'s argument — which `sure` takes from
    /// `--store-dir` and from nowhere else — or by [`Paths::from_roots`], the
    /// injection point for callers that keep their own locations.
    Caller,
}

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
    origin: Origin,
}

impl Paths {
    /// Where SURE stores things for the user running it.
    ///
    /// The default, and the whole of it: a caller who names nothing gets the
    /// platform's own per-user locations, and this is the function every
    /// undocumented path in SURE goes through to get them.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::Unavailable`] if the platform does not report a
    /// per-user location for the data or configuration directory. That is rare,
    /// and it is not recoverable by guessing — see the variant's own message.
    pub fn discover() -> Result<Self, PathError> {
        Self::discover_at(None)
    }

    /// The same, with the store's directory named by the caller if they named
    /// one.
    ///
    /// `named` is the directory the store lives in for this run — `sure.db` goes
    /// inside it — and it is the caller's own word: `sure` fills it from
    /// `--store-dir`, an option in the process's argument vector, and from
    /// nowhere else. See the module documentation for why no file and no
    /// environment variable can stand in for it.
    ///
    /// The user-level *settings* directory is not moved. Only the store — the
    /// evidence and history — can be relocated, because that is the thing a
    /// caller has a reason to point at a location of their own, and moving the
    /// settings as well would silently change which configuration is in force.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotAbsolute`] for a relative or empty `named`, before
    /// anything else can happen: a relative path is resolved against the current
    /// directory, so whether it landed inside the project would depend on where
    /// SURE was started. Returns [`PathError::Unavailable`] if the platform
    /// reports no configuration location.
    pub fn discover_at(named: Option<&Path>) -> Result<Self, PathError> {
        let config = dirs::config_dir()
            .ok_or(PathError::Unavailable {
                what: "its user-level settings",
            })?
            .join(APP_DIR);
        match named {
            None => {
                let data = dirs::data_local_dir().ok_or(PathError::Unavailable {
                    what: "its evidence and history",
                })?;
                Self::validated(data.join(APP_DIR), config, Origin::Platform)
            }
            Some(directory) => Self::validated(store_directory(directory)?, config, Origin::Caller),
        }
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
        Self::validated(data, config, Origin::Caller)
    }

    /// Where this run's store location came from.
    #[must_use]
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Check locations once, so that every constructor above is the same rule.
    fn validated(data: PathBuf, config: PathBuf, origin: Origin) -> Result<Self, PathError> {
        absolute("The evidence and history directory", data)
            .and_then(|data| {
                absolute("The user-level settings directory", config).map(|config| (data, config))
            })
            .map(|(data, config)| Self {
                data,
                config,
                origin,
            })
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

    /// The local record store.
    ///
    /// In the data directory rather than the configuration directory, because
    /// history is machine-local: a roaming profile should carry the user's
    /// settings between machines, and should not carry the evidence one machine
    /// gathered about projects it has and the other does not.
    #[must_use]
    pub fn store_file(&self) -> PathBuf {
        self.data.join(STORE_FILE)
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

/// Accept a store directory a caller named, in the caller's own words.
///
/// The one rule for the `named` argument of [`Paths::discover_at`], exposed so
/// that a command line can refuse a relative location *before* it runs anything,
/// with the same implementation the run-time resolution uses rather than a
/// second copy of the rule that could drift from it. The text of
/// [`PathError::NotAbsolute`] is what both report.
///
/// # Errors
///
/// Returns [`PathError::NotAbsolute`] if `named` is relative or empty.
pub fn store_directory(named: &Path) -> Result<PathBuf, PathError> {
    absolute("The store directory", named.to_path_buf())
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
    fn a_named_store_directory_replaces_the_platforms_and_is_marked_as_the_callers() {
        // The two halves of the mechanism in one test, because they are one
        // fact: a caller who names a directory gets *that* directory and the
        // report can say so, and a caller who names nothing gets the platform's
        // own — which is checked here against `dirs` rather than against a
        // second call to the same function.
        let named = rooted(&["work", "store"]);
        let paths = Paths::discover_at(Some(&named)).expect("an absolute store directory");

        assert_eq!(paths.data_dir(), named);
        assert_eq!(paths.store_file(), named.join(STORE_FILE));
        assert_eq!(paths.origin(), Origin::Caller);

        // The settings do not move with it. A caller who points the store
        // somewhere else has not asked SURE to read a different configuration.
        let platform = Paths::discover().expect("this machine reports per-user locations");
        assert_eq!(paths.config_dir(), platform.config_dir());
        assert_eq!(paths.user_config_file(), platform.user_config_file());

        // And naming nothing is still the platform's own location, marked as
        // such: this is what `sure doctor` with no `--store-dir` reports.
        assert_eq!(Paths::discover_at(None).unwrap(), platform);
        assert_eq!(platform.origin(), Origin::Platform);
        assert_eq!(
            platform.data_dir(),
            dirs::data_local_dir()
                .expect("this machine reports a local data directory")
                .join(APP_DIR)
        );
        assert_eq!(
            platform.config_dir(),
            dirs::config_dir()
                .expect("this machine reports a configuration directory")
                .join(APP_DIR)
        );
    }

    #[test]
    fn a_named_store_directory_is_held_to_the_same_rule_as_a_discovered_one() {
        // A location the caller chose is not privileged: the reason the store
        // may not be inside the project is that the judged thing can edit it,
        // which is just as true of a directory the caller named.
        let project = rooted(&["work", "project"]);
        let paths = Paths::discover_at(Some(&project.join(PROJECT_CACHE_DIR)))
            .expect("an absolute store directory");

        assert!(matches!(
            paths.ensure_outside(&project),
            Err(PathError::InsideProject { .. })
        ));
    }

    #[test]
    fn a_named_store_directory_that_is_relative_or_empty_is_refused() {
        // Same rule as the roots: an empty location is what an unfilled field
        // looks like, and a relative one would be resolved against whatever
        // directory SURE happened to be started in.
        for text in [".sure", "", "."] {
            let error = Paths::discover_at(Some(Path::new(text))).unwrap_err();
            assert!(
                matches!(error, PathError::NotAbsolute { .. }),
                "{text:?} was accepted as a store directory"
            );
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

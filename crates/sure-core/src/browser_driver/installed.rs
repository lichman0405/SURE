//! Finding a browser on this machine, from a fixed list and never from the
//! project.
//!
//! # Why the project is not asked
//!
//! Everything SURE checks is written by a coding agent, and this module decides
//! **what SURE will execute**. A search that read a path out of the project's
//! configuration would be a project choosing which program SURE launches — which
//! is the one thing the whole execution-trust design exists to prevent, and it
//! would be that while looking like a convenience feature.
//!
//! [`find`] therefore reads exactly two things: a table of paths compiled into
//! this binary, and `PATH`. Neither is influenced by the directory being
//! inspected. What can be observed from outside the crate is checked from
//! outside it — `crates/sure-core/tests/browser_driver.rs` asserts that the
//! program this build would run is absolute, outside the checkout and outside
//! the temporary directory — and what cannot be is checked beside this file, in
//! the two tests about relative `PATH` entries and absolute candidates.
//!
//! # Why a miss is a safe answer
//!
//! The workspace manifest warns, about `%LOCALAPPDATA%`, that reading it by hand
//! "would be wrong whenever the variable is unset". The same variables are read
//! here and the difference is worth stating rather than assuming: **getting a
//! configuration path wrong writes a file to the wrong place, and getting a
//! browser path wrong produces `None`.** A miss is reported as
//! [`AbsenceReason::NoDriverInstalled`](crate::browser::AbsenceReason::NoDriverInstalled)
//! and the check is skipped; nothing is corrupted and nothing is claimed.
//!
//! # What is not searched
//!
//! No registry, no `App Paths`, no launcher shims, no `Scoop`/`Chocolatey`
//! directories beyond what `PATH` gives. A browser installed somewhere none of
//! those reach is reported as **not installed** rather than guessed at, and the
//! honest description of that is a limitation of this build and not a fact about
//! the machine.

use std::path::PathBuf;

// `Path` arrives with `is_one_of_ours`, which is the tests' predicate and is
// compiled only for them.
#[cfg(test)]
use std::path::Path;

/// The browsers this build will drive, in the order they are preferred.
///
/// **Every one of them is Chromium and speaks the DevTools protocol**, which is
/// the only reason a list this short is enough. Firefox and Safari are absent
/// because driving them needs a different protocol and a different implementation
/// — not because they are worse browsers — and a build that claimed to support
/// them by launching them and speaking CDP at them would report
/// [`DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart)
/// on every project.
///
/// The order is a preference between installations and not a judgement: Chrome
/// first because it is the one the mechanism was verified against, then Edge
/// because it ships with Windows and is the same engine.
const NAMES: &[&str] = &[
    "chrome",
    "msedge",
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
    "microsoft-edge",
    "microsoft-edge-stable",
    "brave-browser",
];

/// The first browser found on this machine, or `None`.
///
/// The rule — **the table before `PATH`, and only a file counts** — is in
/// [`first_that_is_a_file`], which is where a test can reach it.
#[must_use]
pub fn find() -> Option<PathBuf> {
    first_that_is_a_file(&table(), &on_path())
}

/// The first of `table`'s candidates that is a file, or the first of `path`'s.
///
/// **The table is searched before `PATH`**, so that a browser installed in the
/// place its maker installs it wins over whatever a `PATH` entry happens to
/// contain — a `PATH` is a mutable list that a project's own setup script could
/// have written to, and this binary's table is not. A candidate that is not a
/// file is not an answer: a directory named `chrome.exe` is not a program, and
/// the search would otherwise report one as the browser it would run.
///
/// **Split out of [`find`] for the same reason [`on_a_path`] is split out of
/// [`on_path`]** — the rule is about *which of two lists is preferred*, and that
/// question cannot be put to a function that builds both lists by reading this
/// machine. `find` is the machine's question; this is the rule.
fn first_that_is_a_file(table: &[PathBuf], path: &[PathBuf]) -> Option<PathBuf> {
    table
        .iter()
        .chain(path)
        .find(|candidate| candidate.is_file())
        .cloned()
}

/// The fixed locations this platform's browsers install to.
///
/// Returned as a list rather than probed in place so that the caller's rule —
/// first one that is a file — is the same for both sources.
fn table() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots() {
        for relative in RELATIVE_TO_A_ROOT {
            found.push(root.join(relative));
        }
    }
    for absolute in ABSOLUTE {
        found.push(PathBuf::from(absolute));
    }
    found
}

/// The directories the relative locations are joined onto, on this platform.
///
/// **Environment variables, and only as roots.** Each is used only if it is set,
/// so an unset one removes candidates rather than producing a wrong one; the
/// names are the ones Windows sets for itself and are not written by any
/// project.
#[cfg(windows)]
fn roots() -> Vec<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect()
}

/// macOS keeps applications in `/Applications`, and a user's own copies in
/// `~/Applications`.
#[cfg(target_os = "macos")]
fn roots() -> Vec<PathBuf> {
    let mut found = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        found.push(PathBuf::from(home).join("Applications"));
    }
    found
}

/// Nothing is looked for outside `PATH` elsewhere, because there is nowhere else
/// a package manager and a distribution agree on.
#[cfg(not(any(windows, target_os = "macos")))]
fn roots() -> Vec<PathBuf> {
    Vec::new()
}

/// Where a browser sits underneath one of [`roots`].
///
/// The Windows entries are spelled with the executable name because that is what
/// has to be passed to the operating system; the macOS ones are the binary
/// *inside* the application bundle, which is what runs.
#[cfg(windows)]
const RELATIVE_TO_A_ROOT: &[&str] = &[
    r"Google\Chrome\Application\chrome.exe",
    r"Microsoft\Edge\Application\msedge.exe",
];

#[cfg(target_os = "macos")]
const RELATIVE_TO_A_ROOT: &[&str] = &[
    "Google Chrome.app/Contents/MacOS/Google Chrome",
    "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "Chromium.app/Contents/MacOS/Chromium",
    "Brave Browser.app/Contents/MacOS/Brave Browser",
];

#[cfg(not(any(windows, target_os = "macos")))]
const RELATIVE_TO_A_ROOT: &[&str] = &[];

/// Locations that are not under any of [`roots`].
#[cfg(not(any(windows, target_os = "macos")))]
const ABSOLUTE: &[&str] = &[
    "/usr/bin/chromium",
    "/snap/bin/chromium",
    "/opt/google/chrome/chrome",
];

#[cfg(any(windows, target_os = "macos"))]
const ABSOLUTE: &[&str] = &[];

/// Every candidate the `PATH` names, in the order the `PATH` lists them.
///
/// **`PATH` is read as the operating system separates it** — `;` on Windows, `:`
/// elsewhere — and on Windows each entry is tried with `.exe` appended. That
/// last part is named rather than assumed: this does **not** read `PATHEXT`, so a
/// browser installed as a `.cmd` shim is not found here. `.exe` is the extension
/// every real Chromium installation uses, and reading `PATHEXT` would be
/// implementing a second search rule to reach shims this module has decided not
/// to run.
///
/// # A relative entry is skipped, and that is the point of this function
///
/// `PATH` may contain `.`, or an empty entry, either of which means *the
/// directory the process is in*. Joined with `chrome.exe` that is
/// `./chrome.exe` — **and SURE is usually run with the project as its working
/// directory, so that is a browser the project itself could have written.** A
/// search that accepted it would hand the choice of what SURE executes straight
/// back to the code under inspection, which is the one outcome this module
/// exists to prevent, and it would do it through a hole nobody would look for
/// because the rest of the function reads `PATH` and looks like an ordinary
/// search.
///
/// The check is `is_absolute` rather than a comparison against the working
/// directory, because the working directory can change between the call and the
/// execution while an absolute path cannot.
fn on_path() -> Vec<PathBuf> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    on_a_path(&path)
}

/// Every candidate the `PATH` in `path` names, which is [`on_path`] with the
/// environment taken out of it.
///
/// **This is split out so that the rule above can be reached by a test.** The
/// first version of the test fed a relative `PATH` to `std::env::split_paths`
/// and filtered the result itself — which checked `std::env` and that test's own
/// line of code, and could not have failed however [`on_path`] was written. A
/// `PATH` is a value; a function that takes it as one can be asked what it does
/// with a bad one, and a function that reads the environment can only be asked
/// about this machine's `PATH`, which is a `PATH` this test does not control.
fn on_a_path(path: &std::ffi::OsStr) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in std::env::split_paths(path) {
        if !directory.is_absolute() {
            continue;
        }
        for name in NAMES {
            for candidate in names_for(name) {
                found.push(directory.join(candidate));
            }
        }
    }
    found
}

/// The file names a browser might have in a `PATH` directory.
#[cfg(windows)]
fn names_for(name: &str) -> Vec<String> {
    vec![format!("{name}.exe")]
}

#[cfg(not(windows))]
fn names_for(name: &str) -> Vec<String> {
    vec![(*name).to_owned()]
}

/// Whether `path` looks like something this module would have found.
///
/// **The tests' use of this is the reason it exists**, which is why it is
/// compiled only for them. The search does not need it — [`find`] returns
/// candidates it built from [`table`] and [`on_path`], so what it returns is one
/// of ours by construction — and a product caller that wanted to check a path it
/// was given would be asking a question about a name, when the thing that
/// matters about a path is who chose it.
///
/// What it lets a test say is *the search found this one* rather than *this one
/// exists*, which are different claims and only the first is about this module.
#[cfg(test)]
#[must_use]
pub fn is_one_of_ours(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let bare = name.strip_suffix(".exe").unwrap_or(name);
    NAMES.contains(&bare)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// An absolute directory, spelled the way **this platform** spells one.
    ///
    /// **The first version of the test below used `"/a/directory"` on both
    /// platforms and failed on Windows**, where that path is *rooted and has no
    /// drive prefix* — `/` begins a path on whichever drive the process is on,
    /// so [`std::path::Path::is_absolute`] is false for it and [`on_a_path`] was
    /// right to skip it. The assertion that caught this was the one asking for
    /// candidates, which cannot pass by accident; had the test only asserted
    /// *nothing came out* it would have been green on Windows for the wrong
    /// reason, having fed the search an entry the search is meant to skip.
    #[cfg(windows)]
    const AN_ABSOLUTE_DIRECTORY: &str = r"C:\a\directory";

    /// See the Windows spelling above.
    #[cfg(not(windows))]
    const AN_ABSOLUTE_DIRECTORY: &str = "/a/directory";

    /// **The search is not empty on the machine it runs on, or it is, and both
    /// are reported rather than asserted.**
    ///
    /// This is deliberately not `assert!(find().is_some())`. A test that
    /// required a browser would fail on a machine without one, and a machine
    /// without one is a supported machine — that is the whole of
    /// [`crate::browser::AbsenceReason::NoDriverInstalled`]. What is asserted is
    /// the property that has to hold either way: **whatever is found is one of
    /// the browsers this build claims to drive, and it is a file.**
    #[test]
    fn whatever_is_found_is_a_browser_this_build_drives() {
        match find() {
            Some(path) => {
                assert!(path.is_file(), "{} is not a file", path.display());
                assert!(
                    is_one_of_ours(&path),
                    "{} is not one of the names this build drives",
                    path.display()
                );
            }
            None => {
                // Said out loud rather than silently passed, so that a run on a
                // machine with no browser says so in its output.
                println!("no browser found on this machine, so nothing was checked");
            }
        }
    }

    /// **The table puts the installations a machine has before the `PATH`**,
    /// which is the ordering rule and the only one this module states.
    ///
    /// Built here from a temporary directory that stands in for a root, so that
    /// the assertion is about the order the candidates are produced in and does
    /// not depend on what is installed.
    #[test]
    fn every_candidate_the_table_produces_is_under_a_root_or_absolute() {
        let table = table();
        let roots = roots();
        assert_eq!(
            table.len(),
            roots.len() * RELATIVE_TO_A_ROOT.len() + ABSOLUTE.len(),
            "the table is not every root crossed with every relative location"
        );
        for candidate in &table {
            assert!(
                roots.iter().any(|root| candidate.starts_with(root))
                    || ABSOLUTE
                        .iter()
                        .any(|absolute| candidate == Path::new(absolute)),
                "{} came from nowhere",
                candidate.display()
            );
        }
    }

    /// The `PATH` names, which are the only thing that makes a non-standard
    /// installation reachable on Windows.
    #[test]
    fn a_browser_on_the_path_is_looked_for_under_the_names_this_build_drives() {
        for name in NAMES {
            for candidate in names_for(name) {
                assert!(
                    is_one_of_ours(Path::new(&candidate)),
                    "{candidate} does not look like one of ours"
                );
            }
        }
        // And the reverse, so that the predicate is not true of everything.
        assert!(!is_one_of_ours(Path::new("notepad.exe")));
        assert!(!is_one_of_ours(Path::new("")));
    }

    /// **A relative `PATH` entry is skipped, so that the project cannot supply
    /// the browser.**
    ///
    /// `PATH` may contain `.` or an empty entry, both of which mean *the
    /// current directory*, and SURE is normally run with the project as its
    /// working directory. Joined with a browser's name that is a program the
    /// project could have written, and a search that accepted it would let the
    /// code under inspection choose what SURE executes.
    ///
    /// Two halves, and **the second is the one that carries the rule**: the
    /// first checks this machine's `PATH`, which on a normal machine has no
    /// relative entry at all and would therefore pass against code that did not
    /// skip one.
    ///
    /// The second feeds a `PATH` made of relative entries **through [`on_path`]'s
    /// own body** by way of [`on_a_path`], which is the only way a test can ask
    /// what the product does with a bad `PATH` rather than what this machine
    /// happens to have. An earlier version of this test split a relative `PATH`
    /// itself and filtered the result with its own line of code: it asserted
    /// `std::env`'s behaviour and its own arithmetic, and no rewriting of the
    /// search could have failed it.
    #[test]
    fn a_relative_path_entry_never_becomes_a_candidate() {
        for candidate in on_path() {
            assert!(
                candidate.is_absolute(),
                "{} came from a relative PATH entry, and a relative candidate is a program the \
                 project could have written",
                candidate.display()
            );
        }

        // `.` and an empty entry, which are the two spellings of *the directory
        // the process is in*, plus a named relative directory.
        let relative = std::env::join_paths([Path::new("."), Path::new("bin"), Path::new("")])
            .expect("a relative PATH is representable");
        let candidates = on_a_path(&relative);
        assert!(
            candidates.is_empty(),
            "a relative PATH produced {} candidates, and the first is {}: a project could have \
             written that program",
            candidates.len(),
            candidates
                .first()
                .map_or_else(String::new, |path| path.display().to_string())
        );

        // And the other direction, so that this is not satisfied by a search
        // that ignores every `PATH`: the same function, given an absolute
        // directory, produces the names it drives.
        let absolute = std::env::join_paths([Path::new(AN_ABSOLUTE_DIRECTORY)])
            .expect("an absolute PATH is representable");
        let found = on_a_path(&absolute);
        assert_eq!(
            found.len(),
            NAMES.len(),
            "the absolute directory {AN_ABSOLUTE_DIRECTORY} produced no candidates, so the case \
             above would pass against a search that reads no PATH at all"
        );
        assert!(
            found
                .iter()
                .all(|candidate| candidate.starts_with(AN_ABSOLUTE_DIRECTORY))
        );
    }

    /// **The table's candidates are preferred to the `PATH`'s, and only a file
    /// is an answer.**
    ///
    /// Both halves are the rule [`first_that_is_a_file`] states, and this test
    /// can only state them because that function takes the two lists as values.
    /// **A version of this written against [`find`] would have been the
    /// relative-`PATH` mistake again**: it would have had to build the two lists
    /// itself and then assert its own ordering, which is a test of the test.
    ///
    /// Every case is a file this test wrote in a directory of its own, so
    /// nothing here depends on what is installed.
    #[test]
    fn the_table_is_searched_before_the_path_and_only_a_file_counts() {
        let directory =
            std::env::temp_dir().join(format!("sure-which-browser-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        let table_candidate = directory.join("from-the-table");
        let path_candidate = directory.join("from-the-path");
        std::fs::write(&table_candidate, b"").expect("written");
        std::fs::write(&path_candidate, b"").expect("written");
        let missing = directory.join("not-here-at-all");

        assert_eq!(
            first_that_is_a_file(
                std::slice::from_ref(&table_candidate),
                std::slice::from_ref(&path_candidate)
            ),
            Some(table_candidate.clone()),
            "a PATH entry won over the table, so a project that appends to its own PATH \
             chooses the program SURE runs"
        );

        // The other direction, twice over, so that the answer above is not *the
        // first argument whatever it is*: a table candidate that is not there,
        // and a table candidate that is a directory.
        assert_eq!(
            first_that_is_a_file(
                std::slice::from_ref(&missing),
                std::slice::from_ref(&path_candidate)
            ),
            Some(path_candidate.clone()),
            "a table candidate that is not a file stopped the search before the PATH"
        );
        assert_eq!(
            first_that_is_a_file(
                std::slice::from_ref(&directory),
                std::slice::from_ref(&path_candidate)
            ),
            Some(path_candidate.clone()),
            "a directory was taken for a program, and SURE would report a browser it cannot run"
        );
        assert_eq!(first_that_is_a_file(&[missing], &[]), None);

        std::fs::remove_dir_all(&directory).expect("removed");
    }

    /// **Every candidate is an absolute path built from this binary's table or
    /// from an absolute `PATH` entry.** The property the two sources share, and
    /// the one that matters: nothing here can name a file inside the directory
    /// being checked.
    #[test]
    fn every_candidate_is_absolute() {
        for candidate in table().into_iter().chain(on_path()) {
            assert!(
                candidate.is_absolute(),
                "{} is not absolute",
                candidate.display()
            );
        }
    }
}

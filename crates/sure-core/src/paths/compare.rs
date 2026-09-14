//! Comparing paths without touching the filesystem.
//!
//! The question this module exists to answer is
//! [`is_within`]: *is this path the same as that one, or below it?* It is asked
//! about a location SURE is about to write authoritative evidence to, and about
//! the project being checked, which is controlled by the agent whose work is
//! under evaluation. Getting it wrong in one direction writes evidence inside
//! the working tree, where the agent can edit it, and a fabricated pass becomes
//! indistinguishable from a real one. Getting it wrong in the other direction
//! refuses to write at all, which is a visible failure.
//!
//! **The error is therefore aimed at the visible direction**: when a comparison
//! is genuinely ambiguous, this module reports "within".
//!
//! Three things make the naive version wrong, and each is handled here.
//!
//! 1. **String prefixes are not path prefixes.** `C:\project-evil` starts with
//!    the characters of `C:\project`. Comparison is component by component.
//! 2. **Case.** Windows compares paths case-insensitively, so `C:\Work` and
//!    `c:\work` are one directory. A case-sensitive check reports "outside" for
//!    a path that is inside — the dangerous direction.
//! 3. **Canonicalised paths look different.** `std::fs::canonicalize` on
//!    Windows returns a `\\?\C:\...` verbatim path, which does not compare equal
//!    to the `C:\...` a caller passed in. A check that misses this is defeated
//!    by any caller that canonicalises first, which is the natural thing to do.
//!
//! Nothing here consults the filesystem, so it works for paths that do not exist
//! yet — which is the normal case for a store SURE is about to create.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path};

/// How a platform decides whether two spellings name the same location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseSensitivity {
    /// Two names match only when they are spelled identically.
    Sensitive,
    /// Two names match when they differ only in letter case.
    Insensitive,
}

impl CaseSensitivity {
    /// What this platform does.
    ///
    /// Windows is case-insensitive. macOS is included with it because the
    /// default volume is, and a case-sensitive macOS volume is rare enough that
    /// assuming the opposite would be wrong on the common machine. Getting this
    /// wrong in the *other* direction — calling a case-insensitive platform
    /// sensitive — is what lets a path escape the check, so the assumption is
    /// made on the side of the answer that refuses.
    #[must_use]
    pub const fn platform() -> Self {
        if cfg!(any(windows, target_os = "macos")) {
            Self::Insensitive
        } else {
            Self::Sensitive
        }
    }
}

/// The `..` component, kept as text because it cannot be folded away here.
const PARENT: &str = "..";

/// Whether `candidate` is `root` itself, or below it, under this platform's
/// rules.
#[must_use]
pub fn is_within(candidate: &Path, root: &Path) -> bool {
    is_within_case(candidate, root, CaseSensitivity::platform())
}

/// [`is_within`] with the case rule stated rather than assumed, so both rules
/// can be tested on one machine.
///
/// An empty `root` is never contained in by anything: every path would match it,
/// and a check that accepts everything refuses nothing.
#[must_use]
pub fn is_within_case(candidate: &Path, root: &Path, case: CaseSensitivity) -> bool {
    let root = normalize(root);
    if root.is_empty() {
        return false;
    }
    let candidate = normalize(candidate);
    if candidate.len() < root.len() {
        return false;
    }
    root.iter()
        .zip(candidate.iter())
        .all(|(a, b)| component_eq(a, b, case))
}

/// Whether two paths name the same location.
#[must_use]
pub fn same_path_case(a: &Path, b: &Path, case: CaseSensitivity) -> bool {
    let a = normalize(a);
    let b = normalize(b);
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| component_eq(x, y, case))
}

/// [`same_path_case`] under this platform's rules.
#[must_use]
pub fn same_path(a: &Path, b: &Path) -> bool {
    same_path_case(a, b, CaseSensitivity::platform())
}

fn component_eq(a: &OsString, b: &OsString, case: CaseSensitivity) -> bool {
    match case {
        CaseSensitivity::Sensitive => a == b,
        CaseSensitivity::Insensitive => {
            if a == b {
                return true;
            }
            // `to_string_lossy` is safe to compare with: a name that is not
            // valid Unicode becomes replacement characters on both sides, which
            // can only make two names compare *equal* more often, never less.
            // That is the refusing direction.
            a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        }
    }
}

/// A path as a list of components, with `.` removed, `..` folded where it can
/// be, and the platform's path prefix reduced to one spelling.
///
/// `Component::RootDir` is kept as the component's own text, which is `\` on
/// Windows and `/` elsewhere, so the two platforms are not written as one.
fn normalize(path: &Path) -> Vec<OsString> {
    // The prefix and the root are the anchor: nothing below them can pop them,
    // and `..` applied directly to a root is that same root.
    let mut anchor: Vec<OsString> = Vec::new();
    let mut rest: Vec<OsString> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                if let Some(key) = prefix_key(prefix.kind()) {
                    anchor.push(OsString::from(key));
                }
            }
            Component::RootDir => anchor.push(component.as_os_str().to_owned()),
            Component::CurDir => {}
            Component::ParentDir => match rest.last() {
                // `..` above another `..` names some directory outside this
                // path, and this module cannot know which one.
                Some(last) if last == OsStr::new(PARENT) => rest.push(OsString::from(PARENT)),
                // A component to fold into.
                Some(_) => {
                    rest.pop();
                }
                // Nothing above it. On a rooted path `..` applied to the root
                // is that same root; on a relative one there is no parent to
                // name, so it stays literal rather than matching anything.
                None if anchor.is_empty() => rest.push(OsString::from(PARENT)),
                None => {}
            },
            Component::Normal(_) => rest.push(component.as_os_str().to_owned()),
        }
    }

    anchor.append(&mut rest);
    anchor
}

/// One spelling per prefix, so that a verbatim path and an ordinary one are the
/// same path.
///
/// This is the fix for the third hazard in the module comment. The drive letter
/// is lowercased unconditionally: Windows has no uppercase `C:` distinct from
/// `c:`, so this is not a case-sensitivity decision.
#[cfg(windows)]
fn prefix_key(kind: std::path::Prefix<'_>) -> Option<String> {
    use std::path::Prefix;
    match kind {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
            Some(format!("{}:", char::from(letter).to_ascii_lowercase()))
        }
        // `\\?\UNC\server\share` and `\\server\share` are one location.
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => Some(format!(
            "//{}/{}",
            server.to_string_lossy().to_lowercase(),
            share.to_string_lossy().to_lowercase()
        )),
        // A bare `\\?\` qualifies the path without naming a location. Dropping
        // it is what lets `\\?\C:\a` match `C:\a`.
        Prefix::Verbatim(text) if text.is_empty() => None,
        Prefix::Verbatim(text) => Some(format!("\\\\?\\{}", text.to_string_lossy())),
        Prefix::DeviceNS(name) => Some(format!("\\\\.\\{}", name.to_string_lossy())),
    }
}

/// A prefix cannot occur in a path on this platform. The arm exists because
/// `Component::Prefix` is part of the type on every platform.
#[cfg(not(windows))]
fn prefix_key(_kind: std::path::Prefix<'_>) -> Option<String> {
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn within(candidate: &str, root: &str) -> bool {
        is_within_case(
            Path::new(candidate),
            Path::new(root),
            CaseSensitivity::Sensitive,
        )
    }

    #[test]
    fn a_path_is_within_itself() {
        assert!(within("/a/b", "/a/b"));
    }

    #[test]
    fn a_path_below_another_is_within_it() {
        assert!(within("/a/b/c", "/a/b"));
    }

    #[test]
    fn a_sibling_is_not_within() {
        assert!(!within("/a/c", "/a/b"));
        assert!(!within("/b", "/a"));
    }

    #[test]
    fn a_child_is_not_within_its_parent() {
        assert!(!within("/a", "/a/b"));
    }

    #[test]
    fn a_shared_string_prefix_is_not_a_path_prefix() {
        // The bug this replaces: `starts_with` on the text says yes.
        assert!("/project-evil".starts_with("/project"));
        assert!(!within("/project-evil", "/project"));
        assert!(!within("/a/bb", "/a/b"));
    }

    #[test]
    fn a_trailing_separator_does_not_change_the_answer() {
        assert!(within("/a/b/", "/a"));
        assert!(within("/a/b", "/a/"));
        assert!(within("/a/", "/a"));
    }

    #[test]
    fn a_repeated_separator_does_not_change_the_answer() {
        assert!(within("/a//b", "/a"));
    }

    #[test]
    fn a_current_directory_component_is_ignored() {
        assert!(within("/a/./b", "/a"));
        assert!(within("/a/b", "/a/./"));
    }

    #[test]
    fn a_parent_component_is_folded_where_it_can_be() {
        assert!(within("/a/b/../b/c", "/a/b"));
        assert!(within("/a/../a/b", "/a"));
        assert!(!within("/a/../b", "/a"));
    }

    #[test]
    fn a_parent_component_that_cannot_be_folded_is_not_a_wildcard() {
        // `..` with nothing above it names some directory outside this path,
        // and this module cannot know which. Treating it as matching anything
        // would let any path be reported as within any other.
        assert!(within("../a", ".."));
        assert!(!within("../a", "/b"));
        assert!(!within("/a", ".."));
    }

    #[test]
    fn a_path_above_a_root_is_the_root() {
        // `/..` is `/`. Folding it to nothing would leave `/..` unmatched
        // against `/`, and reporting a path that is the root as outside itself
        // is the wrong direction.
        assert!(within("/a", "/.."));
        assert!(within("/", "/.."));
    }

    #[test]
    fn an_empty_root_refuses_everything() {
        // Otherwise the check accepts every candidate, which is the same as not
        // having it.
        assert!(!within("/a/b", ""));
        assert!(!within("", ""));
    }

    #[test]
    fn a_relative_candidate_is_compared_against_a_relative_root() {
        assert!(within("a/b", "a"));
        assert!(!within("a/b", "a/c"));
    }

    #[test]
    fn case_is_ignored_when_the_platform_ignores_it() {
        let insensitive = CaseSensitivity::Insensitive;
        assert!(is_within_case(
            Path::new("/Work/Project/Evidence"),
            Path::new("/work/project"),
            insensitive
        ));
        assert!(!is_within_case(
            Path::new("/Work/Project/Evidence"),
            Path::new("/work/project"),
            CaseSensitivity::Sensitive
        ));
    }

    #[test]
    fn case_folding_does_not_change_the_component_structure() {
        // Folding the whole path as one string would make this match.
        assert!(!is_within_case(
            Path::new("/a/Bc/d"),
            Path::new("/a/b"),
            CaseSensitivity::Insensitive
        ));
    }

    #[test]
    fn non_ascii_names_compare_by_their_text() {
        assert!(within("/配置/证据", "/配置"));
        assert!(is_within_case(
            Path::new("/配置/证据"),
            Path::new("/配置"),
            CaseSensitivity::Insensitive
        ));
        assert!(!within("/配置", "/配置/证据"));
    }

    #[test]
    fn a_space_in_a_name_is_just_a_character() {
        assert!(within("/Program Files/SURE data", "/Program Files"));
        assert!(!within("/Program Files (x86)", "/Program Files"));
    }

    #[test]
    fn the_same_path_predicate_agrees_with_within() {
        let case = CaseSensitivity::Insensitive;
        assert!(same_path_case(Path::new("/a/B"), Path::new("/a/b"), case));
        assert!(same_path_case(
            Path::new("/a/b/"),
            Path::new("/a/./b"),
            case
        ));
        assert!(!same_path_case(
            Path::new("/a/b"),
            Path::new("/a/b/c"),
            case
        ));
        assert!(!same_path_case(Path::new("/a/b"), Path::new("/a"), case));
    }

    #[test]
    fn the_platform_rule_is_whichever_keeps_the_check_conservative() {
        // Asserting the rule rather than the platform: on a case-insensitive
        // platform this must be `Insensitive`, because the other answer is the
        // one that lets a path escape.
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(CaseSensitivity::platform(), CaseSensitivity::Insensitive);
        } else {
            assert_eq!(CaseSensitivity::platform(), CaseSensitivity::Sensitive);
        }
    }

    #[cfg(windows)]
    mod windows {
        use super::*;

        #[test]
        fn a_drive_letter_has_no_case() {
            // Even under the case-*sensitive* policy: a drive letter is not a
            // name whose case distinguishes anything on Windows.
            assert!(is_within_case(
                Path::new(r"c:\work\project\evidence"),
                Path::new(r"C:\work\project"),
                CaseSensitivity::Sensitive,
            ));
        }

        #[test]
        fn different_drives_are_never_within_each_other() {
            // The root components differ, so no amount of matching below them
            // can make these the same location.
            assert!(!within(r"D:\work", r"C:\work"));
            assert!(!within(r"C:\work", r"D:\work"));
        }

        #[test]
        fn a_verbatim_path_matches_the_ordinary_spelling() {
            // `std::fs::canonicalize` produces the verbatim form, so a caller
            // that canonicalises before checking would otherwise defeat it.
            assert!(within(r"\\?\C:\work\project\evidence", r"C:\work\project"));
            assert!(within(r"C:\work\project\evidence", r"\\?\C:\work\project"));
        }

        #[test]
        fn a_verbatim_unc_path_matches_the_ordinary_spelling() {
            assert!(within(
                r"\\?\UNC\server\share\project\evidence",
                r"\\server\share\project"
            ));
        }

        #[test]
        fn a_drive_relative_path_is_not_within_an_absolute_one() {
            // `C:work` is relative to the current directory on `C:`, which is
            // not known here, so it cannot be reported as inside `C:\work`.
            assert!(!within(r"C:work", r"C:\work"));
            assert!(!within(r"C:\work", r"C:work"));
        }

        #[test]
        fn a_parent_of_a_drive_root_is_that_root() {
            assert!(within(r"C:\work", r"C:\.."));
            assert!(within(r"C:\", r"C:\.."));
        }

        #[test]
        fn a_unc_share_root_behaves_like_a_drive_root() {
            assert!(within(r"\\server\share\a", r"\\server\share"));
            assert!(!within(r"\\server\other\a", r"\\server\share"));
        }
    }

    #[cfg(unix)]
    mod unix {
        use super::*;

        /// The part of the default entry point's answer that is the same on
        /// every Unix, so it is asserted on every Unix.
        ///
        /// `is_within`, not `within`: the shared tests take the case rule as an
        /// argument, so this is the one place that checks the platform's own
        /// answer reaches the function callers actually use.
        #[test]
        fn a_path_outside_by_more_than_case_is_outside() {
            assert!(is_within(
                Path::new("/home/u/project/evidence"),
                Path::new("/home/u/project")
            ));
            assert!(!is_within(
                Path::new("/home/other/evidence"),
                Path::new("/home/u/project")
            ));
        }

        /// Linux's answer, asserted where Linux's answer holds.
        ///
        /// This used to be one test under a bare `#[cfg(unix)]`, which is not
        /// one answer: `unix` includes macOS, whose default volume is
        /// case-insensitive, so that test asserted Linux's rule on a platform
        /// whose rule is the opposite and failed there. "Unix" is a set of
        /// platforms and not a case rule — `CaseSensitivity::platform` already
        /// says so, and this now follows it rather than contradicting it.
        #[test]
        #[cfg(not(target_os = "macos"))]
        fn the_default_entry_point_folds_nothing_on_a_case_sensitive_platform() {
            assert!(!is_within(
                Path::new("/home/u/Project/evidence"),
                Path::new("/home/u/project")
            ));
        }

        /// macOS's answer, asserted where macOS's answer holds — the same path
        /// as above, and the opposite expectation.
        #[test]
        #[cfg(target_os = "macos")]
        fn the_default_entry_point_folds_case_on_a_case_insensitive_platform() {
            assert!(is_within(
                Path::new("/home/u/Project/evidence"),
                Path::new("/home/u/project")
            ));
        }

        #[test]
        fn a_leading_slash_is_required_for_an_absolute_match() {
            assert!(!within("home/u/.sure", "/home/u"));
        }
    }
}

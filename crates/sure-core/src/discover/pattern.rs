//! Resolving a pattern a *project wrote* against the walk SURE already has.
//!
//! Three ecosystems now declare their members with a list of paths, two of them
//! with globs in it: `package.json`'s `workspaces`, `pnpm-workspace.yaml`, and
//! Cargo's `[workspace] members`. The resolution is here rather than in each
//! module because **two ecosystems that expand `*` differently would report two
//! different workspaces for the same directory tree**, and neither module could
//! tell you which one was wrong.
//!
//! # The two things this refuses, and why refusing beats answering
//!
//! `**` is not expanded. It matches any number of directories, so honouring it
//! means walking an unbounded part of the tree; *not* honouring it while saying
//! nothing would drop every member under it, which is how a workspace comes to
//! look smaller than it is — and a smaller workspace is one a check is then run
//! against. The refusal is a statement about the pattern and is a
//! [`UnresolvedReason`], not an empty list.
//!
//! A pattern that would leave the project is refused too. A workspace pattern is
//! project-controlled text, and this is the same containment rule the
//! fingerprint applies to the paths Git reports: only [`Component::Normal`] is
//! accepted, so an absolute path, a root, `.` and `..` cannot appear. **What
//! "absolute" means is the platform's idea and not a spelling** — `C:\Windows`
//! is refused on Windows and is a single ordinary file name on Unix, where a
//! backslash is a legal character in a name. Accepting it there is correct and a
//! test asserting the Windows answer everywhere failed on the macOS and Ubuntu
//! jobs before that sentence existed.
//!
//! [`Component::Normal`]: std::path::Component::Normal

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::paths::CaseSensitivity;

use super::read::{self, Tree};

/// A pattern that named nothing SURE could resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    /// The pattern, verbatim, as data.
    pub pattern: String,
    /// Why.
    pub reason: UnresolvedReason,
}

/// Why a pattern did not resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedReason {
    /// The pattern uses a form SURE does not expand.
    ///
    /// `**` matches any number of directories, and SURE reports it rather than
    /// expanding it. Expanding it means walking an unbounded part of the tree to
    /// produce a member list; *not* expanding it while saying nothing would make
    /// every member under a `**` pattern silently missing, which is then what a
    /// check would be run against.
    UnsupportedPattern,
    /// The pattern names a place that is not inside the project.
    ///
    /// A workspace pattern is project-controlled text, so this is also the
    /// containment rule: `"../../elsewhere"` is refused rather than joined onto
    /// the project root and read.
    NotInsideProject,
    /// The pattern named no directory the walk found.
    ///
    /// **Check [`crate::discover::Discovery::is_complete`] before reporting this
    /// as "there is no such directory".** A walk that ran out of depth or budget
    /// reports nothing at the path it did not reach, and this is what that looks
    /// like from here.
    NoMatch,
}

impl UnresolvedReason {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedPattern => "unsupported_pattern",
            Self::NotInsideProject => "not_inside_project",
            Self::NoMatch => "no_match",
        }
    }

    /// The sentence a person reads.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::UnsupportedPattern => {
                "SURE does not expand this kind of pattern, so it did not list what \
                 this one names."
            }
            Self::NotInsideProject => {
                "This pattern names a place outside the project, so SURE did not \
                 follow it."
            }
            Self::NoMatch => "This pattern named no directory SURE found.",
        }
    }
}

/// Resolve every pattern, keeping unresolved ones as data rather than as errors.
///
/// The root is seeded as already-seen, so a pattern that names it — `"."`, or a
/// `*` that reaches it — does not make the root a member of itself. Each
/// ecosystem describes its root separately, and a root listed twice would also
/// be *read* twice against a finite budget.
pub(super) fn resolve(
    tree: &Tree<'_>,
    patterns: &[String],
    case: CaseSensitivity,
) -> (Vec<PathBuf>, Vec<Unresolved>) {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    seen.insert(read::lookup_key(std::path::Path::new(""), case));

    let mut ordered: Vec<PathBuf> = Vec::new();
    let mut unresolved: Vec<Unresolved> = Vec::new();
    for pattern in patterns {
        match expand(tree, pattern, case) {
            Err(reason) => unresolved.push(Unresolved {
                pattern: pattern.clone(),
                reason,
            }),
            // A refusal and an empty match are kept apart on purpose: one is a
            // statement about the pattern, the other about the project.
            Ok(paths) if paths.is_empty() => unresolved.push(Unresolved {
                pattern: pattern.clone(),
                reason: UnresolvedReason::NoMatch,
            }),
            Ok(paths) => {
                for path in paths {
                    if seen.insert(read::lookup_key(&path, case)) {
                        ordered.push(path);
                    }
                }
            }
        }
    }
    (ordered, unresolved)
}

/// The directories one pattern names.
///
/// # Errors
///
/// [`UnresolvedReason`] for the two pattern forms that are refused rather than
/// answered. `Ok` with an empty list is *named nothing*, which the caller turns
/// into [`UnresolvedReason::NoMatch`] — the two are kept apart because a refusal
/// is a statement about the pattern and an empty match is a statement about the
/// project.
pub(super) fn expand(
    tree: &Tree<'_>,
    pattern: &str,
    case: CaseSensitivity,
) -> Result<Vec<PathBuf>, UnresolvedReason> {
    let trimmed = pattern.trim().trim_end_matches('/');

    // `"."` and `"./"` name the project root, which is a real answer and not a
    // path that climbs out. Handled before the containment check, which refuses
    // them along with everything else that is not a plain relative path.
    if trimmed.is_empty() || trimmed == "." {
        return Ok(vec![PathBuf::new()]);
    }

    // `**` is answered by the whole pattern being refused rather than by the
    // branch under it being dropped. A pattern that silently expanded to fewer
    // members than it names is how a workspace comes to look smaller than it is.
    if trimmed
        .split(['/', '\\'])
        .any(|component| component.contains("**"))
    {
        return Err(UnresolvedReason::UnsupportedPattern);
    }

    let container = read::contained_relative(trimmed).ok_or(UnresolvedReason::NotInsideProject)?;

    let mut current: Vec<PathBuf> = vec![PathBuf::new()];
    for component in container.components() {
        let std::path::Component::Normal(name) = component else {
            // `contained_relative` returns only `Normal` components, so this is
            // unreachable; refusing rather than unwrapping keeps the guarantee
            // local instead of trusting a function in another module to hold it.
            return Err(UnresolvedReason::NotInsideProject);
        };
        let name = name.to_string_lossy();
        let mut next = Vec::new();
        for prefix in &current {
            if name == "*" {
                next.extend(
                    tree.child_directories(prefix)
                        .iter()
                        .map(|child| (*child).to_path_buf()),
                );
            } else {
                let candidate = prefix.join(name.as_ref());
                if tree.is_directory(&candidate) {
                    next.push(candidate);
                }
            }
        }
        current = next;
    }

    // Deduplicated, because two branches can meet: `packages/*` and a literal
    // `packages/app` are one directory twice, and a member list that named it
    // twice would have SURE reading its manifest twice.
    let mut seen = BTreeSet::new();
    current.retain(|path| seen.insert(read::lookup_key(path, case)));
    Ok(current)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Tests for **this** module, against a real walk.
    //!
    //! The three ecosystems' integration tests cover the reasons — `node.rs` and
    //! `rust.rs` each assert a `**` refusal and an escape refusal through a
    //! manifest — but they reach `expand` through a whole project, and this is
    //! the module where a *project's own text* decides which directories SURE
    //! will read. The containment rule in the module doc is a security boundary,
    //! and a boundary checked only through three callers is a boundary that is
    //! checked three ways and stated nowhere.
    //!
    //! Everything here goes through `scan` and `Tree::of`, the same pair
    //! production uses, so a walk that changed shape fails here rather than being
    //! papered over by a hand-built tree.

    use std::path::Path;

    use super::*;
    use crate::scan::{ScanOptions, scan};

    /// A scratch directory under the workspace's git-ignored `target/tmp`.
    ///
    /// The claiming rules are `sure_testkit::scratch`'s, and this file's history
    /// is why they are the ones they are. Five helpers elsewhere in this
    /// repository removed a fixed scratch path with `let _ = remove_dir_all(..)`
    /// and then treated it as fresh, and on Windows that deletion can fail —
    /// producing a report about a directory that had never been emptied. That is
    /// why nothing here is adopted: a directory is taken with `create_dir`, which
    /// fails when the name is taken, and a directory that is already there is
    /// skipped rather than entered. The one deletion the shared helper does make
    /// is of directories carrying *its own* process's id, once per pool, before
    /// this run has created anything — so what it deletes can only be a previous
    /// process's leftovers, and a deletion that fails leaves a directory nobody
    /// is handed.
    ///
    /// Nothing from the process module here, deliberately:
    /// `discovery_runs_none_of_the_scripts_it_reads` reads this file's text and
    /// refuses that module's name, because the tree must not be able to start a
    /// process. That check sees `#[cfg(test)]` code too, and it caught a first
    /// version of this helper that asked for the process id — which is a true
    /// positive about the file and not about the guarantee, so the helper moved
    /// rather than the check. The process id is asked for in `sure-testkit` now,
    /// which is on the other side of that boundary.
    fn scratch(name: &str) -> PathBuf {
        sure_testkit::scratch::directory("discover pattern", name)
    }

    /// A directory in the fixture.
    fn directory(path: &Path) {
        std::fs::create_dir_all(path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
    }

    /// An empty file in the fixture.
    fn file(path: &Path) {
        std::fs::write(path, b"")
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
    }

    #[test]
    fn a_pattern_that_would_leave_the_project_is_refused_in_every_position() {
        let dir = scratch("climb");
        directory(&dir.join("crates/app"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        // Each of these names a real directory *outside* the fixture if it were
        // joined onto the root and followed, which is the whole reason the rule
        // exists. `crates/../..` and `crates/app/../../..` are the interesting
        // ones: they begin with a directory that genuinely resolves, so a check
        // that only looked at the first component would pass them.
        for pattern in [
            "..",
            "../elsewhere",
            "crates/../..",
            "crates/app/../../..",
            "./..",
            "crates/app/..",
        ] {
            assert_eq!(
                expand(&tree, pattern, case).unwrap_err(),
                UnresolvedReason::NotInsideProject,
                "{pattern}"
            );
        }

        // A refusal reaches `resolve` as a refusal and not as a pattern that
        // named nothing: the two are different statements, one about the pattern
        // and one about the project, and a reader told the wrong one goes looking
        // in the wrong place.
        let (members, unresolved) = resolve(&tree, &["crates/app/..".to_owned()], case);
        assert!(members.is_empty());
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].pattern, "crates/app/..");
        assert_eq!(unresolved[0].reason, UnresolvedReason::NotInsideProject);
    }

    #[test]
    fn an_absolute_pattern_is_refused_where_this_platform_says_it_is_absolute() {
        let dir = scratch("absolute");
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        // A root is a root on both platforms, and this is the spelling they agree
        // on.
        assert_eq!(
            expand(&tree, "/elsewhere", case).unwrap_err(),
            UnresolvedReason::NotInsideProject
        );

        // `C:\Windows` is the interesting one, and the reason this half is
        // platform-gated: it is an absolute path on Windows and a single ordinary
        // file *name* on Unix, where a backslash is a legal character in a name
        // and there is no drive letter to read. Accepting it there is correct —
        // and a version of this test that asserted the Windows answer everywhere
        // is exactly what failed on the macOS and Ubuntu jobs before the module
        // doc carried the sentence about it.
        let drive = expand(&tree, r"C:\Windows", case);
        if cfg!(windows) {
            assert_eq!(
                drive.unwrap_err(),
                UnresolvedReason::NotInsideProject,
                "an absolute Windows path"
            );
        } else {
            assert_eq!(
                drive.expect("a name, not a path"),
                Vec::<PathBuf>::new(),
                "a Unix file name that happens to contain a backslash"
            );
        }
    }

    #[test]
    fn a_pattern_this_cannot_expand_is_refused_rather_than_partly_expanded() {
        let dir = scratch("deep-glob");
        directory(&dir.join("crates/app/src"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        // `crates/**` is the one that matters. `crates` alone *does* resolve, so
        // an implementation that dropped the unexpandable branch would return one
        // member and say nothing about the directories underneath it — and a
        // workspace that is quietly smaller than it is, is a workspace a check is
        // then run against.
        for pattern in ["**", "crates/**", "crates/**/app", "**/app", "a**b"] {
            assert_eq!(
                expand(&tree, pattern, case).unwrap_err(),
                UnresolvedReason::UnsupportedPattern,
                "{pattern}"
            );
        }

        // The literal reading is still available to a project that means it, so
        // the refusal above is about the *form* and not about the characters.
        assert_eq!(
            expand(&tree, "crates/app", case)
                .expect("a plain path")
                .len(),
            1
        );
    }

    #[test]
    fn the_root_is_named_by_a_dot_and_is_never_a_member_of_itself() {
        let dir = scratch("root");
        directory(&dir.join("crates"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        // `.` is a real answer — the project root — and not a path that climbs
        // out, so it is handled before the containment check rather than by it.
        for pattern in ["", ".", "./", "/"] {
            assert_eq!(
                expand(&tree, pattern, case).expect("the root"),
                vec![PathBuf::new()],
                "{pattern:?}"
            );
        }

        // And `resolve` seeds the root as already seen, so a pattern naming it
        // does not make the project a member of itself. That is not cosmetic: a
        // project listed twice would be *read* twice, against a budget that is
        // finite.
        let (members, unresolved) = resolve(&tree, &[".".to_owned(), "*".to_owned()], case);
        assert!(!members.contains(&PathBuf::new()), "{members:?}");
        assert!(unresolved.is_empty(), "{unresolved:?}");
    }

    #[test]
    fn a_star_names_directories_and_never_files() {
        let dir = scratch("star");
        directory(&dir.join("crates/app"));
        file(&dir.join("crates/README.md"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        let members = expand(&tree, "crates/*", case).expect("a star is expanded");
        assert_eq!(members, vec![Path::new("crates").join("app")]);

        // And naming the file resolves to nothing rather than to the file itself.
        // A workspace member is a directory with a manifest in it, so a file at
        // the path is not a member — and returning it would have SURE reading
        // `crates/README.md/Cargo.toml`.
        assert_eq!(
            expand(&tree, "crates/README.md", case).expect("a plain path"),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn two_patterns_that_reach_one_directory_name_it_once() {
        let dir = scratch("dedup");
        directory(&dir.join("packages/app"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        let (members, unresolved) = resolve(
            &tree,
            &["packages/*".to_owned(), "packages/app".to_owned()],
            case,
        );
        assert_eq!(
            members,
            vec![Path::new("packages").join("app")],
            "{members:?}"
        );
        assert!(unresolved.is_empty(), "{unresolved:?}");
    }

    #[test]
    fn a_refusal_is_told_apart_from_a_pattern_that_named_nothing() {
        let dir = scratch("nothing");
        directory(&dir.join("crates/app"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        // A directory that is not there is `Ok` with an empty list, and not an
        // error: nothing about the pattern is wrong.
        assert_eq!(
            expand(&tree, "packages", case).expect("a plain path"),
            Vec::<PathBuf>::new()
        );

        let (members, unresolved) = resolve(
            &tree,
            &["packages".to_owned(), "crates/**".to_owned()],
            case,
        );
        assert!(members.is_empty(), "{members:?}");
        assert_eq!(unresolved.len(), 2, "{unresolved:?}");
        assert_eq!(unresolved[0].reason, UnresolvedReason::NoMatch);
        assert_eq!(unresolved[1].reason, UnresolvedReason::UnsupportedPattern);
        assert_eq!(unresolved[0].reason.as_str(), "no_match");
        assert_eq!(unresolved[1].reason.as_str(), "unsupported_pattern");
    }

    #[test]
    fn a_pattern_is_matched_by_this_platform_s_case_rule_and_not_by_a_spelling() {
        let dir = scratch("case");
        directory(&dir.join("crates/app"));
        let scan = scan(&dir, ScanOptions::default()).expect("scan the fixture");
        let case = CaseSensitivity::platform();
        let tree = Tree::of(&scan, case);

        // Windows and macOS fold case on their default volumes; Linux does not.
        // The assertion is written against the *rule* rather than against one
        // platform's answer, because a test asserting the Windows answer
        // everywhere is what broke the macOS and Ubuntu jobs once already.
        let found = expand(&tree, "crates/APP", case).expect("a plain path");
        if case == CaseSensitivity::Insensitive {
            assert_eq!(found, vec![Path::new("crates/APP")], "{found:?}");
        } else {
            assert!(found.is_empty(), "{found:?}");
        }
    }

    #[test]
    fn every_reason_a_pattern_can_fail_for_can_be_named_and_described() {
        // The three variants this enum has today. The wire name is what a report
        // carries and the sentence is what a person reads, so a variant missing
        // either is one that reaches a finding as nothing at all.
        for reason in [
            UnresolvedReason::UnsupportedPattern,
            UnresolvedReason::NotInsideProject,
            UnresolvedReason::NoMatch,
        ] {
            assert!(!reason.as_str().is_empty(), "{reason:?}");
            assert!(reason.as_str().is_ascii(), "{reason:?}");
            assert_eq!(
                reason.as_str(),
                reason.as_str().to_lowercase(),
                "{reason:?}"
            );
            assert!(reason.plain_description().ends_with('.'), "{reason:?}");
        }
    }
}

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

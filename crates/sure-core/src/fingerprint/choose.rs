//! Which kind of fingerprint a project gets, and why the answer is not "ask
//! whether it is in a repository".
//!
//! Two kinds exist — [`crate::fingerprint::git_fingerprint`] and
//! [`crate::fingerprint::content_fingerprint`] — and neither can choose itself.
//! This module is the one place that decides, so that the decision is a sentence
//! somebody wrote rather than an accident of which function a caller reached
//! for.
//!
//! # The rule
//!
//! A project is fingerprinted **by Git** when it is at the root of the working
//! tree that contains it. Otherwise it is fingerprinted **by content**.
//!
//! "At the root" is [`Git::prefix`] returning nothing: Git reports a path from
//! the top of the working tree to where the question was asked, and that path is
//! empty exactly when the two are the same directory.
//!
//! # Why the obvious rule is wrong
//!
//! "Is this directory inside a repository?" is the check anyone would write
//! first, and it answers `yes` for a directory that is one component deep in
//! somebody else's checkout. Everything Git would then contribute is about the
//! *other* project:
//!
//! - **`HEAD` is the outer repository's.** The fingerprint digests it, on
//!   purpose — a checkout of a different commit with no local changes is a
//!   different state. For a subdirectory that is exactly backwards: a commit
//!   anywhere else in the repository moves `HEAD`, so the subdirectory's
//!   fingerprint moves although not one of its files changed. The result is
//!   evidence marked stale over and over for a project nobody touched, which is
//!   how a person learns to stop reading the word "stale" — and the document
//!   that says so is `docs/architecture/FINGERPRINTING.md`.
//! - **The change list is scoped, but the state is not.** `-- .` and
//!   `--show-prefix` mean Git reports only changes under the project, which is
//!   right. What they cannot do is make the repository the project's.
//!
//! A content manifest has none of those problems: it reads the project and only
//! the project, so nothing outside can move it. It costs more — every readable
//! file is read, where Git's kind reads only the changed ones — and that is the
//! deliberate direction. Correctness of the verdict comes before performance,
//! and the failure being avoided is a fingerprint that moves when the project
//! did not.
//!
//! # The case that is left as an error, on purpose
//!
//! When Git cannot be started at all — [`FingerprintError::GitUnavailable`] —
//! this returns that error rather than falling back to a content fingerprint.
//!
//! The temptation is obvious and the reason to refuse is the whole point of
//! having one place that decides: **the kind a project gets must be a function
//! of the project, not of the machine.** Falling back would mean the same
//! directory, unchanged, produced a `Git` fingerprint on a developer's laptop
//! and a `Content` fingerprint on a build agent without Git. Those two values
//! are not equal — [`ProjectFingerprint::matches`] compares the kind — so every
//! stored result would be reported as stale on the other machine, and a project
//! checked in both places would never agree with itself.
//!
//! A caller who knows the project is not a repository, or wants the content
//! manifest for a reason of their own, calls
//! [`crate::fingerprint::content_fingerprint`] directly. That is the escape
//! hatch, and it is explicit on purpose: it is a decision, so somebody makes it.
//!
//! [`Git::prefix`]: crate::fingerprint::Git
//! [`ProjectFingerprint::matches`]: sure_domain::vocabulary::ProjectFingerprint::matches

use std::path::Path;

use sure_domain::vocabulary::ProjectFingerprint;

use super::FingerprintOptions;
use super::content::content_fingerprint;
use super::error::FingerprintError;
use super::git::Git;

/// Fingerprint a project, choosing the kind by looking at the project.
///
/// # Errors
///
/// Everything [`content_fingerprint`] returns, plus anything
/// [`Git::fingerprint`] returns. [`FingerprintError::GitUnavailable`] is
/// **not** turned into a content fingerprint; see the module comment for why.
///
/// # Examples
///
/// ```no_run
/// use sure_core::fingerprint::{FingerprintOptions, project_fingerprint};
///
/// # fn main() -> Result<(), sure_core::fingerprint::FingerprintError> {
/// let fingerprint = project_fingerprint(
///     std::path::Path::new(r"C:\work\project"),
///     &FingerprintOptions::default(),
/// )?;
/// // Which kind the project called for, and the value itself.
/// println!("{} {}", fingerprint.kind.as_str(), fingerprint.digest);
/// # Ok(())
/// # }
/// ```
pub fn project_fingerprint(
    root: &Path,
    options: &FingerprintOptions,
) -> Result<ProjectFingerprint, FingerprintError> {
    fingerprint_with(&Git::system(), root, options)
}

/// The same, with the Git to ask named by the caller.
///
/// Separate from [`project_fingerprint`] so that a test can supply a Git that is
/// not installed, which is the only way to exercise the branch that refuses
/// rather than falling back. It is not `pub` because naming a Git is not
/// something a caller of SURE should be doing; the product asks the one on the
/// path.
pub(crate) fn fingerprint_with(
    git: &Git,
    root: &Path,
    options: &FingerprintOptions,
) -> Result<ProjectFingerprint, FingerprintError> {
    match git.prefix(root) {
        // Not in a working tree, according to the only authority on the
        // question. This is the ordinary case the content kind exists for.
        Err(FingerprintError::NoRepository { .. }) => content_fingerprint(root, options),
        // At the top of the working tree: the repository is this project's, so
        // Git's answer is about this project and nothing else.
        Ok(prefix) if prefix.as_os_str().is_empty() => git.fingerprint(root, options),
        // Inside a working tree, but not at its top. The repository belongs to
        // somebody else — see the module comment.
        Ok(_) => content_fingerprint(root, options),
        // Git is not installed, the root is not absolute, or something else went
        // wrong. Every one of those is a refusal; none of them is a reason to
        // answer a different question.
        Err(other) => Err(other),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The branch that cannot be reached from an integration test.
    ///
    /// Every other outcome of the match above is decided by the *project* — is
    /// it in a repository, and is it at the top of one — and a fixture can build
    /// each of those. This one is decided by the *machine*, and the only way to
    /// see it is to name a Git that is not installed. `Git::with_program` is
    /// public for that reason; [`fingerprint_with`] is not, which is why this
    /// test is here rather than beside the other chooser tests in
    /// `tests/fingerprint_content.rs`.
    #[test]
    fn a_machine_without_git_is_refused_rather_than_read_by_content() {
        let git = Git::with_program("sure-no-git-is-installed-on-this-machine");
        let root = std::env::temp_dir();
        let error = fingerprint_with(&git, &root, &FingerprintOptions::default()).expect_err(
            "a Git that cannot start is not a reason to answer a different question \
             about the project",
        );
        assert!(
            matches!(error, FingerprintError::GitUnavailable { .. }),
            "expected GitUnavailable, got {error}"
        );
    }
}

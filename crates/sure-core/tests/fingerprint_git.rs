//! Fingerprinting a real project in a real Git repository.
//!
//! P2-T002 acceptance:
//!
//! > HEAD/dirty tracked diff/relevant untracked state affect fingerprint.
//! > Fingerprint changes on relevant source changes.
//!
//! Both sentences are about a value that has to come out **the same** for the
//! same project state and **different** for a different one, and neither half
//! can be checked without a filesystem, a repository and a real `git`. So every
//! test here builds one and drives the product's own entry point,
//! [`sure_core::fingerprint::git_fingerprint`].
//!
//! # Why real Git rather than a recorded status
//!
//! The parser has its own unit tests over synthetic bytes, and they are the
//! right place for the records Git almost never emits. What they cannot show is
//! whether this build of Git, on this platform, with this configuration, says
//! what the parser expects. That gap is exactly where the `--relative` failure
//! lived: a flag whose documentation promises one thing, that produced an empty
//! change list and a successful exit, and that no synthetic test could have
//! caught. Everything here goes through `git` as the machine's own Git.
//!
//! # What this file cannot test, and says so
//!
//! A test that passes because its premise did not hold is worse than no test, so
//! the cases below are marked rather than quietly skipped. Each is recorded in
//! `docs/architecture/FINGERPRINTING.md` under "Known coverage gaps", with the
//! mutation that corresponds to it.
//!
//! **A symbolic link.** The [`Contents::Link`] path — a link is recorded by its
//! target and never read through — is covered on Unix. It is *not* covered on
//! Windows, and the reason is a fact about this machine rather than an oversight:
//! `CreateSymbolicLinkW` needs Developer Mode or administrator rights, and this
//! development machine has neither. Git's own answer to the same problem is
//! `core.symlinks=false`, under which a link is written as an ordinary file
//! holding its target — so on such a machine there is no link in the working
//! tree for SURE to find, and the fingerprint is right to hash what is there.
//!
//! **A file that cannot be read.** [`FingerprintError::Unreadable`] is produced
//! by the Unix test and its arm is therefore exercised. On Windows it is not:
//! there is no mode that stops an owner reading their own file short of an ACL,
//! and setting one would change state outside the fixture. The Windows arm is
//! the same code with a different [`std::io::ErrorKind`].
//!
//! **`assume-unchanged` and `skip-worktree`.** Not covered anywhere. Both are a
//! person instructing Git to report a file as unchanged, and SURE believes the
//! answer Git gives — see gap 3 in the document above. The behaviour is
//! deliberate; what is missing is a test pinning it as deliberate, and it is
//! named here rather than left to look like coverage.
//!
//! **A pipe, a socket or a device at a tracked path.** Covered on Unix by
//! `a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang`, which is
//! the platform where it can hang: `File::open` on a FIFO with no writer blocks
//! until one appears. It is *not* covered on Windows, and not for want of
//! trying — a pipe is not a filesystem entry there, so Git cannot report one as
//! a tracked path and the arm in `Reader::read` is unreachable. Two facts about
//! Git were measured while writing the test rather than assumed, and both are
//! easy to get backwards:
//!
//! - A tracked path whose working-tree entry is replaced by a pipe **is**
//!   reported, as an ordinary modified file (`.M`). That is why the fixture
//!   commits the file before replacing it.
//! - An *untracked* pipe is **not** reported at all — it does not appear in
//!   `git status --porcelain=v2 -uall` output. Nothing may be concluded from its
//!   absence from a fingerprint, because it is absent from Git's answer first.
//!
//! **`core.symlinks=false`.** Described above and not exercised: the fixture
//! sets the machine's own default rather than overriding it, so the branch where
//! a link arrives as a file holding its target has no test on either platform.
//!
//! [`Contents::Link`]: sure_core::fingerprint::git
//! [`FingerprintError::Unreadable`]: sure_core::fingerprint::FingerprintError

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::fingerprint::{FingerprintError, FingerprintOptions, Git, git_fingerprint};
use sure_core::paths::CaseSensitivity;
use sure_core::scan::{ScanOptions, scan};
use sure_core::vocabulary::{FingerprintKind, ProjectFingerprint};

/// How many entries a fixture's walks are allowed before a test is wrong.
const FIXTURE_LIMIT: usize = 5_000;

/// A scratch Git repository that removes itself.
///
/// Under the workspace's own `target/` rather than the system temp directory, so
/// the fixture is on the same volume as the checkout. The fixture directory's
/// name contains a space and a non-ASCII character, which is the cheapest way to
/// make every path in every test below a path two platforms disagree about.
struct Repo {
    path: PathBuf,
}

impl Repo {
    /// A repository with one commit in it, or a panic saying what Git said.
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure 指纹 git")
            .join(unique);
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));

        let repo = Self { path };
        repo.git(&["init", "-b", "main"]);
        // Set per repository rather than read from the machine, so a fixture
        // does not depend on the developer's global configuration and does not
        // write to it. `commit.gpgsign` is off because a machine with signing
        // configured would otherwise stop every commit here at a passphrase
        // prompt that no test can answer.
        for (key, value) in [
            ("user.email", "sure@example.invalid"),
            ("user.name", "SURE test fixture"),
            ("commit.gpgsign", "false"),
            // The fixture's own line-ending rule. This machine has
            // `core.autocrlf=true` globally, and a fixture whose files are
            // rewritten on checkout would be testing that setting.
            ("core.autocrlf", "false"),
            // Long paths on Windows. A developer checking out a Node project
            // has this set; without it `git status` cannot even describe the
            // fixture in [`a_path_longer_than_the_old_windows_limit_is_fingerprinted`].
            ("core.longpaths", "true"),
            ("gc.auto", "0"),
        ] {
            repo.git(&["config", key, value]);
        }
        repo
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Run Git in the fixture, and fail loudly rather than carry on.
    fn git(&self, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(arguments)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap_or_else(|error| {
                panic!(
                    "these tests need Git on the path (trying to run {arguments:?}): {error}\n\n\
                     Without Git there is nothing here to test: the product answers \
                     `GitUnavailable` and every assertion below would be about the absence \
                     of Git rather than about a fingerprint."
                )
            });
        assert!(
            output.status.success(),
            "git {arguments:?} failed with {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Write a file, creating its directory.
    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.path.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    /// Make a directory.
    fn mkdir(&self, relative: &str) -> &Self {
        let full = self.path.join(relative);
        std::fs::create_dir_all(&full)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", full.display()));
        self
    }

    /// Stage everything and commit it.
    fn commit(&self, message: &str) -> &Self {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-m", message]);
        self
    }

    /// The same, with the author and committer dates fixed.
    ///
    /// Git commits are identified by their contents *including* the timestamp,
    /// so two commits of the same tree made in the same second are the same
    /// commit. That is a fact about Git and it decides how
    /// [`two_different_commits_of_one_tree_are_two_fingerprints`] has to be
    /// written: it needs two commits it can tell apart, which means two dates.
    fn commit_at(&self, message: &str, date: &str) -> &Self {
        self.git(&["add", "-A"]);
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(["commit", "-m", message])
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap_or_else(|error| panic!("cannot run git commit: {error}"));
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        self
    }

    /// The fingerprint of the fixture under default options.
    fn fingerprint(&self) -> ProjectFingerprint {
        self.fingerprint_with(FingerprintOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }

    fn fingerprint_with(
        &self,
        options: FingerprintOptions,
    ) -> Result<ProjectFingerprint, FingerprintError> {
        git_fingerprint(self.path(), &options)
    }

    /// The options every test here uses: the defaults, with a fixture-sized
    /// budget so a fixture that escaped its root fails quickly.
    fn options() -> FingerprintOptions {
        FingerprintOptions::default()
            .with_scan(ScanOptions::default().with_max_entries(FIXTURE_LIMIT))
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        // Windows keeps handles open longer than Unix does, so a `git` process
        // may still hold one.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A repository with one committed source file and one committed test file.
fn small(test: &str) -> Repo {
    let repo = Repo::new(test);
    repo.write("src/main.rs", "fn main() {}\n")
        .write(
            "src/lib.rs",
            "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
        )
        .write("README.md", "# A project\n")
        .commit("first");
    repo
}

/// Two fingerprints, and whether they name the same project state.
fn same(a: &ProjectFingerprint, b: &ProjectFingerprint) -> bool {
    a.matches(b)
}

// ---------------------------------------------------------------------------
// The shape of a fingerprint
// ---------------------------------------------------------------------------

#[test]
fn a_clean_repository_has_a_fingerprint_and_says_nothing_has_changed() {
    let repo = small("clean");
    let fingerprint = repo.fingerprint();

    assert_eq!(fingerprint.kind, FingerprintKind::Git);
    assert_eq!(
        fingerprint.digest.len(),
        64,
        "a SHA-256 digest is 64 hex digits"
    );

    let state = fingerprint
        .git
        .expect("a Git fingerprint carries Git state");
    assert_eq!(state.head.len(), 40, "a full commit id");
    assert!(!state.dirty);
    // Neither half is present, and `None` is not the same as a digest of
    // nothing: "this project is clean" and "this project has an empty change
    // list" are two states, and only one of them is true here.
    assert_eq!(state.dirty_digest, None);
    assert_eq!(state.untracked_digest, None);
    assert_eq!(state.branch.as_deref(), Some("main"));
}

#[test]
fn the_same_project_state_fingerprints_to_the_same_value_every_time() {
    // The property everything downstream depends on. Without it every result is
    // stale on the next run, and the word stops meaning anything.
    let repo = small("repeat");
    let first = repo.fingerprint();
    let second = repo.fingerprint();

    assert!(
        same(&first, &second),
        "{} != {}",
        first.digest,
        second.digest
    );
    // The identity is fresh each time and the digest is not, which is the
    // distinction that makes `matches` compare the right field.
    assert_ne!(first.id, second.id);
}

#[test]
fn two_checkouts_of_the_same_state_agree() {
    // Two directories, two repositories, one project. The files are written in
    // different orders on purpose: if anything in the digest depended on the
    // order a filesystem returned names, or on the order Git listed changes,
    // these two would differ.
    let one = Repo::new("agree-one");
    let two = Repo::new("agree-two");
    for (repo, order) in [(&one, false), (&two, true)] {
        if order {
            repo.write(
                "src/lib.rs",
                "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
            )
            .write("src/main.rs", "fn main() {}\n");
        } else {
            repo.write("src/main.rs", "fn main() {}\n").write(
                "src/lib.rs",
                "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
            );
        }
        // A fixed date on both, so the two commits really are one commit. Left
        // to the clock they would usually agree and occasionally not, which is
        // the kind of test that fails in a colleague's session once a month.
        repo.commit_at("first", "2020-01-01T00:00:00+00:00");
    }

    // The commits come out identical — same tree, same message, same author,
    // same second — which is a premise this test needs and checks rather than
    // assumes. If it ever fails, the fixtures differ in some other way and the
    // assertion below is no longer about what it says it is.
    let (first, second) = (one.fingerprint(), two.fingerprint());
    assert_eq!(
        first.git.as_ref().unwrap().head,
        second.git.as_ref().unwrap().head,
        "the two fixtures were supposed to be the same commit"
    );
    assert_eq!(
        first.digest, second.digest,
        "the same working tree produced two fingerprints"
    );
    assert!(same(&first, &second));
}

#[test]
fn two_different_commits_of_one_tree_are_two_fingerprints() {
    // The deliberate cost of digesting HEAD, asserted so that it is a decision.
    // Two commits with byte-identical trees are the same project *content* and
    // different project *states*: one is the version somebody checked out and
    // the other is not. Evidence produced against one is not evidence about the
    // other, so the fingerprints must differ — and they do, even though every
    // file a check can read is identical.
    //
    // The tree is compared and asserted equal, because without that this test
    // would pass for the wrong reason the moment the two commits drifted apart.
    let repo = Repo::new("same-tree");
    repo.write("src/main.rs", "fn main() {}\n")
        .commit_at("first", "2020-01-01T00:00:00+00:00");
    let first = repo.fingerprint();

    repo.git(&["commit", "--allow-empty", "-m", "second"]);
    // An empty commit made through `git` directly would carry the machine's
    // clock; the tree is what matters here and it is unchanged either way.
    let second = repo.fingerprint();

    assert_eq!(
        repo.git(&["rev-parse", "HEAD^{tree}"]).trim(),
        repo.git(&["rev-parse", "HEAD~1^{tree}"]).trim(),
        "the two commits were supposed to have the same tree"
    );
    assert_ne!(
        first.git.as_ref().unwrap().head,
        second.git.as_ref().unwrap().head
    );
    assert!(
        !same(&first, &second),
        "a checkout of a different commit kept the same fingerprint"
    );
}

// ---------------------------------------------------------------------------
// The acceptance criteria: HEAD, the dirty diff, and untracked files
// ---------------------------------------------------------------------------

#[test]
fn a_different_commit_is_a_different_fingerprint() {
    let repo = small("head");
    let before = repo.fingerprint();

    repo.write(
        "src/lib.rs",
        "pub fn add(a: u32, b: u32) -> u32 { a + b }\n// and\n",
    )
    .commit("second");
    let after = repo.fingerprint();

    assert!(!same(&before, &after));
    assert_ne!(
        before.git.unwrap().head,
        after.git.unwrap().head,
        "the commits are supposed to differ"
    );
}

#[test]
fn an_edited_tracked_file_is_a_different_fingerprint() {
    let repo = small("dirty");
    let before = repo.fingerprint();

    repo.write(
        "src/lib.rs",
        "pub fn add(a: u32, b: u32) -> u32 { a + b + 1 }\n",
    );
    let after = repo.fingerprint();

    assert!(!same(&before, &after));
    let state = after.git.expect("Git state");
    assert!(state.dirty);
    assert!(
        state.dirty_digest.is_some(),
        "a dirty tree has a tracked digest"
    );
    assert_eq!(state.untracked_digest, None, "nothing new was added");
}

#[test]
fn a_new_untracked_file_is_a_different_fingerprint() {
    let repo = small("untracked");
    let before = repo.fingerprint();

    repo.write("src/extra.rs", "pub fn extra() {}\n");
    let after = repo.fingerprint();

    assert!(!same(&before, &after));
    let state = after.git.expect("Git state");
    assert!(state.untracked_digest.is_some());
    assert_eq!(
        state.dirty_digest, None,
        "no tracked file changed, so there is no tracked digest"
    );
}

#[test]
fn a_deleted_tracked_file_is_a_different_fingerprint() {
    // A deletion leaves a path Git names and a filesystem that has nothing at
    // it. That is neither "an empty file" nor "a file I could not read", and the
    // digest has to tell it from both.
    let repo = small("deleted");
    let before = repo.fingerprint();

    std::fs::remove_file(repo.path().join("src/lib.rs")).unwrap();
    let after = repo.fingerprint();

    assert!(!same(&before, &after));
    assert!(after.git.unwrap().dirty);
}

#[test]
fn a_deleted_file_and_an_empty_file_are_two_states() {
    // The other half of the test above, and the one it cannot reach on its own.
    // There, the deleted file is not empty, so "gone" and "empty" are told apart
    // by their contents whatever the code does. Here the two projects differ in
    // nothing else: same commit, same single tracked path, and the file either
    // deleted or emptied. A digest that wrote "no file" as "a file of no bytes"
    // would call these two the same project, and the deletion of a real file
    // would read as a change of no consequence.
    //
    // Two fixtures rather than two states of one, because neither end state can
    // be reached from the other without passing through a state that also
    // differs: emptying a file and then deleting it leaves a second change
    // behind. `commit_at` gives both the same HEAD so that the comparison is
    // about the working tree and not about two commits.
    let deleted = Repo::new("gone-not-empty");
    let emptied = Repo::new("empty-not-gone");
    for repo in [&deleted, &emptied] {
        repo.write("src/f.rs", "x")
            .commit_at("first", "2020-01-01T00:00:00+00:00");
    }

    std::fs::remove_file(deleted.path().join("src/f.rs")).unwrap();
    emptied.write("src/f.rs", "");

    let (gone, empty) = (deleted.fingerprint(), emptied.fingerprint());
    assert_eq!(
        gone.git.as_ref().unwrap().head,
        empty.git.as_ref().unwrap().head,
        "the two fixtures must start from one commit, or this compares HEADs"
    );
    assert!(
        !same(&gone, &empty),
        "a deleted file and an empty file are two different projects"
    );
}

#[test]
fn an_untracked_file_that_is_rewritten_with_the_same_bytes_is_not_a_change() {
    // Content, not modification time. A file rewritten with what it already said
    // has said nothing new, and a fingerprint that moved would mark every result
    // stale on a build that rewrote a generated file identically.
    let repo = small("same-bytes");
    repo.write("scratch.txt", "the same words\n");
    let before = repo.fingerprint();

    repo.write("scratch.txt", "the same words\n");
    let after = repo.fingerprint();

    assert!(same(&before, &after), "the project did not change");
}

#[test]
fn a_file_put_back_the_way_it_was_is_the_same_fingerprint() {
    // The other direction, and the one that matters more: an edit that is
    // undone leaves the project exactly as it was, so evidence produced before
    // the edit is evidence about the project as it is now. A fingerprint that
    // remembered *that* something changed rather than *what* the project is
    // would mark that evidence stale for ever.
    let repo = small("undone");
    let before = repo.fingerprint();

    repo.write("src/lib.rs", "something else entirely\n");
    assert!(!same(&before, &repo.fingerprint()));

    repo.write(
        "src/lib.rs",
        "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
    );
    assert!(
        same(&before, &repo.fingerprint()),
        "putting the file back did not put the fingerprint back"
    );
}

#[test]
fn the_two_halves_of_a_dirty_tree_are_digested_separately() {
    // Two projects can differ in one half and agree in the other, and a single
    // combined digest would have to say which. Keeping them apart is what lets a
    // report say *what* changed rather than only *that* something did.
    let repo = small("halves");

    repo.write(
        "src/lib.rs",
        "pub fn add(a: u32, b: u32) -> u32 { a + b + 1 }\n",
    );
    let tracked_only = repo.fingerprint().git.expect("Git state");

    repo.write("src/extra.rs", "pub fn extra() {}\n");
    let both = repo.fingerprint().git.expect("Git state");

    assert_eq!(tracked_only.dirty_digest, both.dirty_digest);
    assert_ne!(tracked_only.untracked_digest, both.untracked_digest);
    assert!(both.untracked_digest.is_some());
}

// ---------------------------------------------------------------------------
// What is deliberately not in the fingerprint
// ---------------------------------------------------------------------------

#[test]
fn a_change_inside_a_vendor_directory_is_not_a_change() {
    // The rule is that a file is in the fingerprint if and only if a check could
    // read it, and no check reads inside `node_modules`. Including it would mean
    // every reinstall marked every result stale.
    let repo = small("vendor");
    repo.mkdir("node_modules/left-pad");
    repo.write("node_modules/left-pad/index.js", "module.exports = 1;\n");
    let before = repo.fingerprint();

    repo.write("node_modules/left-pad/index.js", "module.exports = 2;\n");
    assert!(
        same(&before, &repo.fingerprint()),
        "an installed dependency changed the fingerprint"
    );
}

#[test]
fn a_tracked_file_in_a_build_directory_is_not_in_the_fingerprint() {
    // The honest cost of the coverage rule, asserted so that it is a decision
    // rather than a surprise. A committed file under `target/` is one no check
    // will ever open, so changing it cannot change any answer SURE gives — and
    // marking results stale for it would be the "fingerprint moved and nothing
    // happened" failure, paid for nothing.
    let repo = Repo::new("tracked-build");
    repo.write("src/main.rs", "fn main() {}\n")
        .write("dist/bundle.js", "// committed by mistake\n")
        .commit("first");

    let before = repo.fingerprint();
    assert!(
        !before.git.as_ref().unwrap().dirty,
        "the fixture should be clean"
    );

    repo.write(
        "dist/bundle.js",
        "// committed by mistake, and now edited\n",
    );
    assert!(
        same(&before, &repo.fingerprint()),
        "a change no check can read changed the fingerprint"
    );
}

#[test]
fn sure_own_cache_inside_the_project_does_not_change_the_fingerprint() {
    // The self-invalidation rule: a fingerprint taken over SURE's own output
    // would change when SURE ran, so checking a project would change the thing
    // being checked and the second run would never match the first.
    let repo = small("own-cache");
    let cache = sure_core::paths::PROJECT_CACHE_DIR;
    repo.mkdir(cache);
    repo.write(
        &format!("{cache}/last-run.json"),
        "{\"verdict\":\"green\"}\n",
    );
    let before = repo.fingerprint();

    repo.write(&format!("{cache}/last-run.json"), "{\"verdict\":\"red\"}\n");
    assert!(same(&before, &repo.fingerprint()));
}

#[test]
fn an_untracked_file_that_git_ignores_is_not_a_change() {
    // Git's answer to "what is untracked" has already applied `.gitignore`, and
    // a file the project says is not part of it is not part of the fingerprint.
    let repo = Repo::new("ignored");
    repo.write(".gitignore", "*.log\n")
        .write("src/main.rs", "fn main() {}\n")
        .commit("first");
    let before = repo.fingerprint();

    repo.write("debug.log", "one\n");
    assert!(same(&before, &repo.fingerprint()));

    repo.write("debug.log", "two, which is longer\n");
    assert!(same(&before, &repo.fingerprint()));
}

#[test]
fn renaming_the_branch_is_not_a_change_to_the_project() {
    // The branch is recorded and not digested. Renaming a branch moves no file,
    // and a fingerprint that changed would mark every result from before the
    // rename stale — which teaches a person to ignore staleness, which is how
    // the other kind of staleness gets missed.
    let repo = small("branch");
    let before = repo.fingerprint();

    repo.git(&["branch", "-m", "renamed"]);
    let after = repo.fingerprint();

    assert_eq!(
        before.digest, after.digest,
        "a branch rename changed the digest"
    );
    assert_eq!(after.git.unwrap().branch.as_deref(), Some("renamed"));
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

#[test]
fn a_path_with_spaces_and_non_ascii_characters_is_fingerprinted_like_any_other() {
    // `git status` escapes a path like this in its default output, and it does
    // not with `-z`. A fingerprint over the escaped form would be a fingerprint
    // of Git's spelling of a name rather than of the name.
    let repo = small("unicode-paths");
    let before = repo.fingerprint();

    repo.write("src/ünïcödé file.rs", "pub fn one() {}\n");
    let added = repo.fingerprint();

    assert!(!same(&before, &added));

    repo.write("src/ünïcödé file.rs", "pub fn two() {}\n");
    assert!(
        !same(&added, &repo.fingerprint()),
        "editing it was not seen"
    );
}

#[test]
fn a_leading_space_is_part_of_a_name() {
    // Windows allows a leading space and not a trailing one, so this is the half
    // of the case that exists on every platform. The parser has unit tests for
    // both; this is the half a filesystem can be asked to prove.
    let repo = small("leading-space");
    let before = repo.fingerprint();

    repo.write(" leading.rs", "pub fn one() {}\n");
    let after = repo.fingerprint();

    assert!(!same(&before, &after));
}

#[test]
fn a_name_that_differs_only_in_case_is_a_build_directory_on_one_platform_and_not_the_other() {
    // The platform's rule, taken as an argument rather than read from the
    // machine, so both answers are tested on one machine — the same device
    // `crate::paths::is_within_case` and the scan's own tests use.
    //
    // The case rule is only observable when the name on disk and the name in the
    // table differ in spelling. A directory called exactly `target` is the build
    // directory under *either* rule, so a fixture with only that one would pass
    // whichever rule was applied and would be testing nothing. Hence `TARGET`,
    // which on Windows is what a project that typed it in capitals actually has
    // — one directory, whose name the filesystem keeps — and on Linux is a
    // directory the project made on purpose.
    let repo = Repo::new("case");
    repo.write("src/main.rs", "fn main() {}\n").commit("first");
    repo.write("TARGET/other.js", "other\n");
    // And a directory whose spelling does match, at a different depth, which is
    // left out under both rules. Put beside `TARGET` rather than inside it,
    // because on Windows one directory cannot hold two names differing in case.
    repo.write("sub/target/exact.js", "exact\n");

    let fingerprint = |case| {
        repo.fingerprint_with(
            Repo::options().with_scan(
                ScanOptions::default()
                    .with_max_entries(FIXTURE_LIMIT)
                    .with_case(case),
            ),
        )
        .unwrap()
    };

    let insensitive = fingerprint(CaseSensitivity::Insensitive);
    assert_eq!(
        insensitive.git.as_ref().unwrap().untracked_digest,
        None,
        "under a case-insensitive rule both target/ and TARGET/ are build directories"
    );

    let sensitive = fingerprint(CaseSensitivity::Sensitive);
    assert!(
        sensitive.git.as_ref().unwrap().untracked_digest.is_some(),
        "under a case-sensitive rule TARGET/ is a directory the project made"
    );
    assert!(!same(&insensitive, &sensitive));

    // The exact-spelling directory is left out under the sensitive rule too, and
    // this is how that is shown rather than asserted: with `TARGET` gone, the
    // only untracked thing left would be `sub/target/exact.js` — and there is
    // none, so a name that matches the table exactly is left out whatever the
    // rule for names that do not.
    std::fs::remove_dir_all(repo.path().join("TARGET")).unwrap();
    assert_eq!(
        fingerprint(CaseSensitivity::Sensitive)
            .git
            .as_ref()
            .unwrap()
            .untracked_digest,
        None,
        "sub/target/exact.js was supposed to be left out by its exact name"
    );
}

#[test]
fn a_path_longer_than_the_old_windows_limit_is_fingerprinted() {
    // 260 characters was the Windows limit for a long time. Three long directory
    // names give a path well past it without needing much depth, so this is the
    // length being tested and not the depth.
    let repo = Repo::new("long-path");
    let segment = "a".repeat(120);
    let deep = format!("{segment}/{segment}/{segment}/buried.rs");
    repo.write(&deep, "fn buried() {}\n").commit("first");
    let before = repo.fingerprint();

    let full = repo
        .path()
        .join(&segment)
        .join(&segment)
        .join(&segment)
        .join("buried.rs");
    assert!(
        full.as_os_str().len() > 260,
        "the fixture is only {} characters long, so it tests nothing",
        full.as_os_str().len()
    );

    repo.write(&deep, "fn buried() { changed }\n");
    let after = repo.fingerprint();

    assert!(!same(&before, &after), "a long path was not fingerprinted");
}

#[test]
fn two_projects_in_one_repository_do_not_change_each_other() {
    // A repository root holding several projects is ordinary, and SURE is
    // pointed at one of them. This is also the test that proves the paths Git
    // reports are being translated from the repository root's view into the
    // project's: if they were not, every changed file would be looked for at
    // `app/app/...`, found missing, and digested as "gone" — under which an edit
    // to a real file would leave the fingerprint alone, because only the path
    // would be hashed and not the contents.
    let repo = Repo::new("monorepo");
    repo.write("app/src/main.rs", "fn main() {}\n")
        .write("app/README.md", "# app\n")
        .write("app-old/src/main.rs", "fn main() {}\n")
        .write("app-old/README.md", "# the old app\n")
        .commit("first");

    let app = repo.path().join("app");
    let options = Repo::options();
    let before = git_fingerprint(&app, &options).unwrap();

    // A sibling changes. The fingerprint of `app` must not.
    repo.write("app-old/src/main.rs", "fn main() { something else }\n");
    repo.write("app-old/new.rs", "pub fn new() {}\n");
    assert!(
        same(&before, &git_fingerprint(&app, &options).unwrap()),
        "a sibling project changed app's fingerprint"
    );

    // A file inside the project changes. It must.
    repo.write("app/src/main.rs", "fn main() { changed }\n");
    let after = git_fingerprint(&app, &options).unwrap();
    assert!(!same(&before, &after), "app's own change was not seen");

    // And a file the sibling added is not in app's untracked half.
    let sibling = git_fingerprint(&repo.path().join("app-old"), &options).unwrap();
    assert!(
        sibling.git.unwrap().untracked_digest.is_some(),
        "the sibling's own new file should be in the sibling's fingerprint"
    );
}

#[test]
fn a_change_inside_a_project_that_is_not_the_repository_root_is_read() {
    // The test above cannot see whether the prefix strip is *right*, only
    // whether it is consistent. Every path it looks at comes from Git as
    // `<prefix>/...`, and an implementation that never stripped the prefix would
    // look each one up at `<root>/<prefix>/...`, find nothing, and digest the
    // path as `Gone` — consistently, on both sides of every comparison there. A
    // file that changes would still change the fingerprint, because a path
    // appearing in the change list at all is a change.
    //
    // What such an implementation loses is the contents. So the assertion here
    // is between two edits of one file: both are `Gone` under that
    // implementation and two different digests under this one.
    let repo = Repo::new("subdir-contents");
    repo.write("app/src/main.rs", "fn main() { one }\n")
        .write("app/README.md", "# app\n")
        .commit("first");
    let app = repo.path().join("app");
    let options = Repo::options();

    repo.write("app/src/main.rs", "fn main() { two }\n");
    let second = git_fingerprint(&app, &options).unwrap();
    repo.write("app/src/main.rs", "fn main() { three }\n");
    let third = git_fingerprint(&app, &options).unwrap();

    assert!(
        !same(&second, &third),
        "two different contents of a file inside this project produced one fingerprint, \
         so the file's bytes are not what is being read"
    );
}

#[test]
fn a_submodule_that_was_never_checked_out_is_a_directory_and_not_a_missing_file() {
    // Git records a gitlink as a mode of its own, and the directory it names
    // need not exist: an uninitialised submodule is exactly that. Treated as a
    // file it would be looked up in the file table instead of the directory
    // table, and `vendor` is in one of them and not the other.
    let repo = Repo::new("gitlink");
    repo.write("src/main.rs", "fn main() {}\n").commit("first");
    let before = repo.fingerprint();

    // An index entry with no working-tree directory and no `.gitmodules`, which
    // is what a submodule looks like before `git submodule update`.
    let empty_tree = repo
        .git(&["hash-object", "-t", "tree", "/dev/null"])
        .trim()
        .to_owned();
    let empty_tree = if empty_tree.is_empty() {
        // Windows has no `/dev/null`; an empty tree is a fixed, well-known hash.
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_owned()
    } else {
        empty_tree
    };
    repo.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{empty_tree},sublayer"),
    ]);

    // The path is named by Git and is not on the filesystem. Whatever SURE
    // decides, it must decide it rather than fail — and it must be a change,
    // because a gitlink that appeared out of nowhere is one.
    let after = repo.fingerprint();
    assert!(!same(&before, &after));
    assert!(after.git.unwrap().dirty);
}

// ---------------------------------------------------------------------------
// Links
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_link_is_recorded_by_its_target_and_not_by_what_it_points_at() {
    // A link is the one place the fingerprint is deliberately wider than the
    // scan: the scan records the link as a loss and moves on, and the
    // fingerprint records where it points. A retargeted link is a change to the
    // repository even though no file's contents moved.
    use std::os::unix::fs::symlink;

    let repo = Repo::new("link-target");
    repo.write("real.txt", "one\n")
        .write("other.txt", "two\n")
        .commit("first");
    symlink("real.txt", repo.path().join("link.txt")).unwrap();
    repo.commit("add the link");
    let before = repo.fingerprint();

    // What the link points at changes: not a change to the fingerprint.
    repo.write("real.txt", "one, but different\n");
    let contents_changed = repo.fingerprint();
    assert!(!same(&before, &contents_changed));

    // Where the link points changes: a change to the fingerprint.
    std::fs::remove_file(repo.path().join("link.txt")).unwrap();
    symlink("other.txt", repo.path().join("link.txt")).unwrap();
    let retargeted = repo.fingerprint();

    assert!(
        !same(&before, &retargeted),
        "retargeting a link is a change to the repository"
    );
}

#[cfg(unix)]
#[test]
fn a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang() {
    // **A repository must not be able to stop a check by existing.**
    //
    // `File::open` on a FIFO with no writer blocks until one appears, and the
    // working tree is written by whoever SURE is checking. A project that
    // commits a file and replaces it with a pipe would therefore hang a
    // fingerprint with no output at all — and a check that never returns cannot
    // be told apart from one that is still working, which is the failure this
    // arm exists to prevent.
    //
    // The premise is Git's, and it was measured rather than assumed: for a
    // tracked path whose working-tree entry is a pipe, `git status
    // --porcelain=v2` reports an ordinary modified file (`.M`), so this path
    // does reach `Reader::read`. An *untracked* pipe is not listed by Git at
    // all, and that is why the fixture commits the file before replacing it.
    use std::sync::mpsc;
    use std::time::Duration;

    let repo = Repo::new("pipe");
    repo.write("script.sh", "#!/bin/sh\n").commit("first");
    let before = repo.fingerprint();

    let named = repo.path().join("script.sh");
    std::fs::remove_file(&named).unwrap();
    let made = Command::new("mkfifo").arg(&named).status();
    // A missing program is a failure and not a skip. Every platform this test
    // is compiled for ships it; if one stops doing so, the premise of the test
    // is gone rather than the behaviour, and a test that passes because its
    // premise did not hold is worse than no test.
    assert!(
        made.as_ref().is_ok_and(std::process::ExitStatus::success),
        "mkfifo is needed to put a pipe in the working tree: {made:?}"
    );

    // Run the fingerprint off this thread so that a blocked `open` arrives as a
    // named failure rather than as a job that sits there until it is killed.
    // The thread is abandoned if this fires, deliberately: it is blocked in a
    // read that will not return, and the process is about to fail anyway.
    let (sender, receiver) = mpsc::channel();
    let root = repo.path().to_path_buf();
    std::thread::spawn(move || {
        let _ = sender.send(git_fingerprint(&root, &Repo::options()));
    });

    let after = match receiver.recv_timeout(Duration::from_secs(60)) {
        Ok(result) => result.unwrap_or_else(|error| panic!("{error}")),
        Err(_) => panic!(
            "fingerprinting a project with a pipe in it did not finish in 60 seconds, \
             so the pipe was opened rather than described"
        ),
    };

    // What is at the path is a pipe, and what was there before was a file. Two
    // states, so two fingerprints — the same decision `Contents::Gone` is on the
    // other side of, where a deletion must not digest as an empty file.
    assert!(
        !same(&before, &after),
        "replacing a tracked file with a pipe is a change to the project"
    );
}

#[cfg(unix)]
#[test]
fn a_change_behind_an_unchanged_link_is_not_a_change() {
    // The documented gap, asserted so that it is a decision with a test rather
    // than a thing to be discovered. Reading through the link would pull in a
    // file the walk never reaches, and the cost is this: content at the other
    // end of an unchanged link can change without the fingerprint moving.
    //
    // It is also the *observable difference* between following a link and not
    // following one, which is why the case is worth a test even though the
    // invisible file would be invisible for a second reason as well: an
    // implementation that read through the link would hash this file's bytes
    // into the link's entry, and this assertion is what would fail.
    //
    // If this test ever starts failing, the behaviour became *safer* and
    // `docs/architecture/FINGERPRINTING.md` is the file that needs updating.
    use std::os::unix::fs::symlink;

    let repo = Repo::new("link-behind");
    repo.write("src/real.rs", "pub fn one() {}\n")
        // A build directory: SURE's walk does not go inside it, and the fixture
        // has no `.gitignore`, so nothing but the fingerprint's own ignore table
        // is keeping this file out.
        .write("dist/generated.js", "// one\n")
        .commit("first");
    symlink("dist/generated.js", repo.path().join("link.txt")).unwrap();
    repo.commit("add the link");

    let before = repo.fingerprint();

    // A target the walk reaches on its own: changing it is a change to the
    // fingerprint, and nothing about the link is why.
    repo.write("src/real.rs", "pub fn one_but_different() {}\n");
    assert!(
        !same(&before, &repo.fingerprint()),
        "a tracked file is a change in its own right, link or no link"
    );

    // The case this test is named for. The link is unchanged and still points
    // where it did; the only way from the project's files to this one is
    // through it; and this one is not a file any check reads.
    let with_target_edited = repo.fingerprint();
    repo.write("dist/generated.js", "// one, but different\n");
    assert!(
        same(&with_target_edited, &repo.fingerprint()),
        "a change behind an unchanged link moved the fingerprint"
    );
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

#[test]
fn a_directory_that_is_not_in_a_repository_has_no_fingerprint() {
    // The system temp directory rather than `target/tmp`, and that is the whole
    // point: a directory under `target/` is inside *this* repository, so Git
    // would find a repository, report the prefix, and SURE would fingerprint the
    // fixture as a subdirectory of SURE. That is correct behaviour and it is not
    // this case.
    let outside = std::env::temp_dir().join(format!(
        "sure-not-a-repo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&outside).unwrap();

    // The premise, checked rather than assumed: if the temp directory is itself
    // inside a repository on this machine, this test would be about something
    // else entirely and should say so instead of passing.
    let inside_a_repository = Command::new("git")
        .arg("-C")
        .arg(&outside)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    assert!(
        !inside_a_repository,
        "the temp directory is inside a Git repository, so this test cannot \
         check what it says it checks: {}",
        outside.display()
    );

    // A file, so the directory is not empty. An empty directory and a directory
    // that does not exist would be one value if SURE were not careful, and this
    // is neither.
    std::fs::write(outside.join("main.rs"), "fn main() {}\n").unwrap();

    match git_fingerprint(&outside, &Repo::options()) {
        Err(FingerprintError::NoRepository { root, message }) => {
            assert_eq!(root, outside);
            assert!(!message.is_empty(), "Git's own words were dropped");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn git_that_will_not_start_is_a_refusal_and_not_an_empty_fingerprint() {
    // Reached by naming a program that cannot exist, which is a better fixture
    // than a fake `git` script: there is no script to keep in step with the real
    // one, and no shell involved in running it.
    let repo = small("no-git");
    let absent = Git::with_program(OsString::from("sure-no-such-program-on-this-machine"));

    match absent.fingerprint(repo.path(), &Repo::options()) {
        Err(FingerprintError::GitUnavailable { program, message }) => {
            assert_eq!(
                program,
                OsString::from("sure-no-such-program-on-this-machine")
            );
            assert!(!message.is_empty());
        }
        other => panic!("expected GitUnavailable, got {other:?}"),
    }
}

#[test]
fn a_relative_root_is_refused_rather_than_resolved_against_the_current_directory() {
    // A fingerprint is a value SURE keeps and compares later, so a root that
    // meant one project today and another tomorrow would make the comparison
    // meaningless. Refused before Git is asked anything.
    match git_fingerprint(Path::new("some/relative/path"), &Repo::options()) {
        Err(FingerprintError::NotAbsolute { root }) => {
            assert_eq!(root, PathBuf::from("some/relative/path"));
        }
        other => panic!("expected NotAbsolute, got {other:?}"),
    }
}

#[test]
fn a_project_with_more_changes_than_the_limit_has_no_fingerprint() {
    // A limit produces an error and never a fingerprint of the part that fitted.
    // The alternative — the value that looks complete and covers half the
    // project — is the failure this whole module exists to refuse.
    let repo = small("too-many");
    for index in 0..6 {
        repo.write(&format!("src/extra-{index}.rs"), "pub fn extra() {}\n");
    }

    match repo.fingerprint_with(Repo::options().with_max_files(3)) {
        Err(FingerprintError::TooManyFiles { limit, .. }) => assert_eq!(limit, 3),
        other => panic!("expected TooManyFiles, got {other:?}"),
    }

    // And the same project under a limit it fits inside has a fingerprint, so
    // the refusal is about the limit and not about the project.
    assert!(
        repo.fingerprint_with(Repo::options().with_max_files(50))
            .is_ok()
    );
}

#[test]
fn a_project_with_more_content_than_the_limit_has_no_fingerprint() {
    let repo = small("too-many-bytes");
    repo.write("src/large.rs", &"// a line of comment\n".repeat(200));

    match repo.fingerprint_with(Repo::options().with_max_bytes(64)) {
        Err(FingerprintError::TooManyBytes { limit, .. }) => assert_eq!(limit, 64),
        other => panic!("expected TooManyBytes, got {other:?}"),
    }

    assert!(
        repo.fingerprint_with(Repo::options().with_max_bytes(1 << 20))
            .is_ok()
    );
}

#[test]
fn the_byte_limit_is_over_the_fingerprint_and_not_over_each_file() {
    // The test above has one large file, so it cannot tell a budget spent over
    // the whole fingerprint from one handed out afresh to every file. Those are
    // different promises: the second reads a project of any size, one file at a
    // time, while the first stops. Each file here is comfortably inside the
    // limit and only the two together are outside it.
    let repo = Repo::new("byte-budget");
    let half = "x".repeat(600);
    repo.write("src/a.rs", &half)
        .write("src/b.rs", &half)
        .commit("first");
    // Both changed, so both are read, and the sum is what matters.
    repo.write("src/a.rs", &format!("{half}1"))
        .write("src/b.rs", &format!("{half}2"));

    let options = Repo::options().with_max_bytes(1_000);
    match repo.fingerprint_with(options) {
        Err(FingerprintError::TooManyBytes { limit, path }) => {
            assert_eq!(limit, 1_000);
            assert_eq!(
                path,
                Path::new("src/b.rs"),
                "the file that crossed the limit is the one to name"
            );
        }
        other => panic!("expected TooManyBytes, got {other:?}"),
    }
}

#[test]
fn a_directory_git_will_not_descend_into_is_walked_and_read() {
    // The only case that reaches the walk inside a fingerprint. Git reports a
    // nested repository as one untracked directory and does not look inside it —
    // it cannot, it is somebody else's repository — so SURE is the one that
    // decides what is in there, and it decides with the same scan a check uses.
    // The files inside are files a check can read, so they are files the
    // fingerprint covers; the alternative is a fingerprint that calls an old
    // result current when somebody edits a checkout that was cloned in.
    let repo = Repo::new("nested-checkout");
    repo.write("src/main.rs", "fn main() {}\n").commit("first");
    let before = repo.fingerprint();

    repo.write("inner/src/lib.rs", "pub fn one() {}\n")
        .write("inner/README.md", "# inner\n");
    // A repository of its own, which is the thing that stops Git descending. The
    // `git` call goes through the fixture helper so that its failure is a
    // message about the fixture rather than a silent difference in behaviour.
    git_init(repo.path().join("inner"));

    // It is a change: files appeared that a check can read.
    let with_inner = repo.fingerprint();
    assert!(
        !same(&before, &with_inner),
        "a nested checkout is part of the project"
    );
    assert!(
        with_inner.git.as_ref().unwrap().untracked_digest.is_some(),
        "Git reports the directory as untracked, and SURE must account for it"
    );

    // Content inside it is read, not just named.
    repo.write("inner/src/lib.rs", "pub fn two() {}\n");
    let edited = repo.fingerprint();
    assert!(
        !same(&with_inner, &edited),
        "a change inside the nested checkout is a change"
    );

    // And a file that moves without its bytes changing is a change too, because
    // the path is in the digest beside the contents. Two files swapping names
    // would otherwise leave every digest of every byte exactly where it was.
    std::fs::rename(
        repo.path().join("inner/README.md"),
        repo.path().join("inner/README2.md"),
    )
    .unwrap();
    assert!(
        !same(&edited, &repo.fingerprint()),
        "a file moved inside the walk kept its fingerprint"
    );
}

#[test]
fn a_walk_that_lost_something_has_no_fingerprint() {
    // The walk inside a fingerprint can come back incomplete: a directory past
    // the depth limit, entries past the budget, something it could not read.
    // Those are files a check would still read, so a tree digest over the part
    // that fitted would claim to cover something nothing looked at — and the
    // fingerprint would then call an old result current for a change inside the
    // part that was skipped.
    //
    // The budget is deliberately tiny rather than the fixture being made large:
    // the case under test is the loss, and a fixture whose incompleteness has to
    // be arranged is one whose premise a reader can check at a glance.
    let repo = Repo::new("incomplete-walk");
    repo.write("src/main.rs", "fn main() {}\n").commit("first");
    repo.write("inner/a.rs", "pub fn a() {}\n")
        .write("inner/b.rs", "pub fn b() {}\n")
        .write("inner/c.rs", "pub fn c() {}\n")
        .write("inner/d.rs", "pub fn d() {}\n");
    git_init(repo.path().join("inner"));

    let options = Repo::options().with_scan(ScanOptions::default().with_max_entries(2));
    match repo.fingerprint_with(options) {
        Err(FingerprintError::IncompleteTree { path, detail }) => {
            assert_eq!(path, Path::new("inner"));
            assert!(
                !detail.is_empty(),
                "the refusal has to say what the walk could not reach"
            );
        }
        other => panic!("expected IncompleteTree, got {other:?}"),
    }

    // And the same project under a budget it fits inside is fingerprinted, so
    // the refusal is about the walk and not about the directory.
    let roomy = Repo::options().with_scan(ScanOptions::default().with_max_entries(FIXTURE_LIMIT));
    assert!(repo.fingerprint_with(roomy).is_ok());
}

/// Make `path` a Git repository, so that a walk from outside it stops there.
fn git_init(path: PathBuf) {
    let output = Command::new("git")
        .arg("-C")
        .arg(&path)
        .args(["init", "-b", "main"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap_or_else(|error| panic!("these tests need Git on the path: {error}"));
    assert!(
        output.status.success(),
        "git init failed in {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn a_change_to_a_file_sure_cannot_read_has_no_fingerprint() {
    // Unix-only, and not a test that skips itself on Windows at runtime.
    // Windows has no `chmod`-style mode that stops the user who owns the file
    // from reading it, and a body behind `#[cfg(unix)]` would compile to
    // nothing there — a test that passes because its premise did not hold,
    // which is worse than no test. The Windows gap is recorded in
    // `docs/architecture/FINGERPRINTING.md`.
    use std::os::unix::fs::PermissionsExt;

    let repo = small("unreadable");
    repo.write("src/secret.rs", "pub fn secret() {}\n")
        .commit("add it");
    let path = repo.path().join("src/secret.rs");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

    let result = repo.fingerprint_with(Repo::options());
    // Restored before asserting, so a failure does not leave a file behind that
    // the fixture's own cleanup cannot remove.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    match result {
        Err(FingerprintError::Unreadable { path: named, .. }) => {
            assert_eq!(named, Path::new("src/secret.rs"));
        }
        other => panic!("expected Unreadable, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The shape of the source, which no run can demonstrate
// ---------------------------------------------------------------------------

/// Every `.rs` file under `crates/`, as (path relative to `crates/`, contents).
///
/// The walk is the product's own scan. That is not a shortcut: a file this test
/// cannot reach is a file the rule below silently does not apply to, and using
/// the scanner means the reach is the same reach everything else in SURE has.
fn rust_sources() -> Vec<(String, String)> {
    let crates = sure_testkit::repository_root().join("crates");
    let walked = scan(&crates, ScanOptions::default()).expect("crates/ is a directory");
    assert!(
        walked.is_complete(),
        "the source tree could not be read completely, so this test would be \
         checking an unknown subset of it"
    );

    walked
        .files()
        .filter(|entry| {
            entry
                .path
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .map(|entry| {
            let path = crates.join(&entry.path);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            (entry.display_path(), text)
        })
        .collect()
}

#[test]
fn git_is_started_in_exactly_one_place() {
    // `RUST_DESIGN.md` asks for one abstraction through which system Git is
    // invoked, and an absence is not something a run can demonstrate: a second
    // call site would work, and every test in this file would still pass. A
    // source check is the only thing that can catch it — the same technique
    // `scan_project.rs` uses to keep the scanner from opening files, and P1-T009
    // used to keep `sure doctor` from reading settings.
    //
    // Doc comments are skipped rather than searched, because they are where the
    // rule itself is written down and a test that forbade mentioning the thing
    // would make the rule impossible to explain.
    //
    // The rule is about code that ships, so the walk is narrowed to `src/`. A
    // test fixture *must* start Git — that is what every test in this file does,
    // and what makes them tests of the real thing rather than of a recording.
    const THE_ONE_PLACE: &str = "sure-core/src/fingerprint/git/mod.rs";
    let shipped: Vec<_> = rust_sources()
        .into_iter()
        .filter(|(path, _)| path.contains("/src/"))
        .collect();
    assert!(
        shipped.len() > 10,
        "the source walk found {} shipped files, which is not this repository — \
         the filter above is matching the wrong thing",
        shipped.len()
    );

    for (path, text) in &shipped {
        if path == THE_ONE_PLACE {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for forbidden in ["Command::new(\"git\")", "Command::new(&self.program)"] {
                assert!(
                    !line.contains(forbidden),
                    "{path}:{} starts Git outside {THE_ONE_PLACE}: {line}",
                    number + 1
                );
            }
        }
    }
}

#[test]
fn the_status_arguments_are_the_ones_the_module_doc_explains() {
    // The flags are the whole of how SURE asks Git a question, and one of them
    // is a flag that must **not** be there. Both halves are pinned here so that
    // changing the invocation is a decision somebody makes on purpose.
    let arguments = Git::STATUS_ARGUMENTS;
    for required in [
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "--untracked-files=all",
        "--no-renames",
        "--",
        ".",
    ] {
        assert!(
            arguments.contains(&required),
            "STATUS_ARGUMENTS is missing {required:?}: {arguments:?}"
        );
    }

    // `--relative` was measured on Git 2.55.0 producing *no output at all* for a
    // working tree with changes in it, with a successful exit — which SURE would
    // read as "this project is clean" and fingerprint as such. It is the exact
    // false green this product exists to prevent, and the reason the prefix is
    // taken off by hand in `relative_to_root` instead.
    assert!(
        !arguments.contains(&"--relative"),
        "`--relative` was measured producing an empty change list on Git 2.55.0; \
         see the note on STATUS_ARGUMENTS before adding it back"
    );

    // One spelling of the untracked rule and not two. `-u` and
    // `--untracked-files` together would be the same flag twice, and whichever
    // Git honoured would be an implementation detail.
    assert_eq!(
        arguments
            .iter()
            .filter(|argument| argument.starts_with("--untracked-files"))
            .count(),
        1
    );
}

#[test]
fn the_settings_that_stop_a_repository_running_a_program_are_still_passed() {
    // A repository is untrusted input, and it carries configuration that Git
    // reads and obeys. Two of those settings name a program — `core.fsmonitor`
    // and `core.pager` — so without these arguments, describing a project is the
    // same act as running whatever the project says to run.
    //
    // This is pinned rather than left to the code because its absence is
    // invisible: on every repository that does not exploit it, removing these
    // changes no output, fails no test, and produces exactly the same
    // fingerprint. The only observable difference is on the repository that was
    // written to exploit it — which is the one case where noticing is too late.
    let arguments = Git::SAFETY_ARGUMENTS;
    for required in ["-c", "core.fsmonitor=false", "--no-pager"] {
        assert!(
            arguments.contains(&required),
            "SAFETY_ARGUMENTS is missing {required:?}: {arguments:?}"
        );
    }

    // Set through `-c` for this invocation and not in the repository. A `-c`
    // argument is read by the Git that is started; a setting written into the
    // project would be a change to the project being checked.
    assert_eq!(
        arguments
            .iter()
            .filter(|argument| argument.starts_with("core."))
            .count(),
        1,
        "a second core.* setting appeared, and the tests above do not describe \
         it: {arguments:?}"
    );

    // Every one of them comes before the subcommand. Git reads `-c` and
    // `--no-pager` as arguments to Git itself, so an `--no-pager` after
    // `status` would be a pathspec naming a file that does not exist rather
    // than a setting.
    for (index, argument) in arguments.iter().enumerate() {
        assert!(
            !argument.starts_with("status"),
            "SAFETY_ARGUMENTS contains a subcommand at {index}: {arguments:?}"
        );
    }
}

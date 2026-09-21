//! `SHA256SUMS.txt` is checked, here, on every platform and in every clone.
//!
//! `P15-T020`'s acceptance is *"Give the checksum manifest a reader, or take it
//! out of the tree."* The file stayed, so this is the reader. Before this file
//! existed nothing in the repository read the manifest:
//!
//! ```text
//! $ grep -rln "SHA256SUMS" . --exclude-dir=target --exclude-dir=.git
//! ./.github/workflows/release.yml          # prose: "not SHA256SUMS.txt"
//! ./crates/sure-testkit/tests/ci_workflow.rs   # a rule about release.yml's text
//! ./docs/development/RELEASE_PROCESS.md    # prose
//! ./progress/HANDOFF.md                    # prose
//! ./progress/state.json                    # prose
//! ./scripts/Assemble-Release.sh            # prose
//! ./scripts/Build-Release.ps1              # prose
//! ./tasks/tasks.json                       # prose about prose
//! ```
//!
//! Every one of those eight names the file; not one of them opens it, parses a
//! digest out of it, or compares it with anything. And the two gates that
//! describe the tree — `node scripts/validate-bootstrap.mjs` and
//! `node scripts/taskctl.mjs validate` — exit 0 over a manifest whose digests
//! are wrong, which is measured rather than asserted at the top of the
//! hand-back for this task. So the file could be arbitrarily stale while every
//! gate this repository runs said the tree was fine, which is the shape of a
//! false green rather than a maintenance lapse.
//!
//! # Where it runs, and why here
//!
//! `cargo test --workspace --all-features` is gate 3 of the six the supervisor
//! runs before an acceptance, and it is line 94 of `.github/workflows/ci.yml`,
//! which runs it on `windows-latest`, `macos-latest` and `ubuntu-latest`. A test
//! in this crate is therefore already in the local gate set *and* already in a CI
//! job, on all three platforms, without a new step in either — and it cannot be
//! forgotten, because there is nothing to remember. `ci_workflow.rs` says the
//! same thing about why the repository-shape checks live in `sure-testkit`.
//!
//! # What it checks, and the one decision in it
//!
//! Every digest is of the **index blob** — the bytes `git commit` will store —
//! and not of the bytes on disk. `.gitattributes` sets `*.ps1 text eol=crlf`, so
//! those two differ for the ten `.ps1` paths this manifest lists, and the choice
//! is not cosmetic: measured at `edb2b00` over a fresh `git clone`, a
//! working-tree reader reports **8 of 195 listed paths stale** that are correct,
//! while an index reader reports 2 — the manifest's own former inconsistency,
//! removed by regenerating it from the index. A reader that is red on every
//! fresh clone is worse than the silence it replaced, and the index form is also
//! the only one that is the same on all three platforms, because the `eol`
//! attributes are applied on checkout and never to the object.
//!
//! The module comment in `sure_testkit::source_manifest` carries the argument
//! and the measurement; `the_check_reads_the_index_blob_and_not_the_working_tree`
//! holds it as a rule.
//!
//! # What this file does not do
//!
//! It does not assert that this machine's working tree differs from its index.
//! That would be a true statement today — the ten `.ps1` paths are split across
//! both forms *in this checkout* — and it would be a red test the day someone
//! renormalised the tree, for a reason that is not a defect. The rule is held
//! with bytes a test chose instead, which is both airtight and independent of
//! what any machine's disk happens to hold.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;

use sure_testkit::source_manifest::{self, Entry, Index, MANIFEST, Manifest};

/// What a contributor is told when the check fails, in the order the steps go.
///
/// Named once rather than repeated in each message: the order is the part that
/// is easy to get wrong, and a message that got it wrong would send a
/// contributor to regenerate digests that the next `git add` invalidates.
const HOW_TO_FIX: &str = "\nstage the files you changed first, then regenerate from the index, then \
                          stage the manifest:\n    git add <what you changed>\n    cargo run -p \
                          sure-testkit --bin source-manifest -- --write\n    git add SHA256SUMS.txt";

/// The index for `paths` at the repository root, as the checker reads it.
fn index_of(root: &Path, paths: &[String]) -> Index {
    Index::read(root, paths).expect(
        "`git` must be runnable to read the index the manifest describes; this check cannot be \
         performed without it, and skipping it would report a pass instead",
    )
}

fn listed_paths(manifest: &Manifest) -> Vec<String> {
    manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect()
}

// --- the check itself ---------------------------------------------------

#[test]
fn the_committed_manifest_matches_the_index() {
    let root = sure_testkit::repository_root();
    let found = source_manifest::verify(&root)
        .expect("the manifest must be readable and so must the index");
    assert!(
        found.is_empty(),
        "{MANIFEST} does not describe the index this commit will write — {} of its entries \
         disagree:\n{}{HOW_TO_FIX}",
        found.len(),
        found
            .iter()
            .map(|finding| format!("  {finding}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_reader_sees_every_listed_path_in_the_index() {
    // Guards the check above from the way it would most plausibly become
    // useless: an index that came back empty would report every path as a
    // finding, and an index read from the wrong directory would report the
    // whole manifest at once. Neither may be mistaken for a clean tree.
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    let paths = listed_paths(&manifest);
    let index = index_of(&root, &paths);
    assert_eq!(
        index.len(),
        paths.len(),
        "the index reader found {} of the {} paths the manifest lists, so this check is not \
         reading the index it claims to",
        index.len(),
        paths.len()
    );
}

#[test]
fn the_check_reads_the_index_blob_and_not_the_working_tree() {
    // A real path, whose bytes on disk are `Cargo.toml`'s and are not these.
    // The checker is handed a manifest and an index and nothing else, so if it
    // agreed with the bytes below it cannot have opened the file.
    const PATH: &str = "Cargo.toml";
    let index_blob = b"the bytes the index holds, which are not the ones on disk\n".to_vec();
    let index_digest = source_manifest::digest_of(&index_blob);

    let agreeing = Manifest {
        entries: vec![Entry {
            digest: index_digest.clone(),
            path: PATH.to_owned(),
        }],
        trailing_newline: true,
    };
    let index = Index::from_blobs(BTreeMap::from([(PATH.to_owned(), index_blob)]));
    assert!(
        source_manifest::findings(&agreeing, &index).is_empty(),
        "a manifest recording the index's own bytes must be accepted"
    );

    // The digest a reader would have produced from the file on disk instead.
    let on_disk = std::fs::read(sure_testkit::repository_root().join(PATH))
        .expect("Cargo.toml is in the checkout");
    let from_the_worktree_digest = source_manifest::digest_of(&on_disk);
    assert_ne!(
        from_the_worktree_digest, index_digest,
        "the two byte strings have to differ, or this test would pass without saying anything"
    );
    let from_the_worktree = Manifest {
        entries: vec![Entry {
            digest: from_the_worktree_digest,
            path: PATH.to_owned(),
        }],
        trailing_newline: true,
    };
    let found = source_manifest::findings(&from_the_worktree, &index);
    assert_eq!(
        found.len(),
        1,
        "a manifest recording the working tree's bytes must be reported: {found:?}"
    );
}

#[test]
fn a_recorded_digest_that_is_wrong_is_reported() {
    // Driven with bytes this test chose rather than with the repository's, so
    // that it says what it says whatever state the tree is in. An assertion
    // about a wrong digest that only held while the tree was correct would stop
    // reporting the defect it is about on the day it matters — which is the
    // shape of a check that is green because it is looking at the wrong thing.
    let zeros = "0".repeat(64);
    let real = b"the bytes the index holds\n".to_vec();
    let manifest = Manifest {
        entries: vec![
            Entry {
                digest: zeros.clone(),
                path: "wrong.txt".to_owned(),
            },
            Entry {
                digest: source_manifest::digest_of(&real),
                path: "right.txt".to_owned(),
            },
        ],
        trailing_newline: true,
    };
    let index = Index::from_blobs(BTreeMap::from([
        ("wrong.txt".to_owned(), real.clone()),
        ("right.txt".to_owned(), real),
    ]));

    let found = source_manifest::findings(&manifest, &index);
    assert_eq!(found.len(), 1, "one wrong digest is one finding: {found:?}");
    assert_eq!(found[0].path(), "wrong.txt");
    match &found[0] {
        source_manifest::Finding::Stale {
            recorded, actual, ..
        } => {
            assert_eq!(*recorded, zeros);
            assert_ne!(
                *actual, zeros,
                "the finding has to carry what was found instead"
            );
        }
        other => panic!("one wrong digest is staleness, not {other:?}"),
    }
}

#[test]
fn corrupting_one_digest_of_the_real_manifest_adds_exactly_one_finding() {
    // The same property against the repository's own index, and written as a
    // difference rather than as a count so that it holds whether or not the
    // committed manifest is currently correct. A version that asserted "one
    // finding" outright would have failed the first time it was run — the
    // manifest was two entries out of date when this file was written — and the
    // tempting repair would have been to weaken it rather than to fix the file.
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    let paths = listed_paths(&manifest);
    let index = index_of(&root, &paths);

    let baseline = source_manifest::findings(&manifest, &index);
    // A path the manifest already disagrees about would swallow the corruption:
    // one stale finding would simply replace another. Corrupt a path that is
    // currently clean, so the difference measures the corruption.
    let mut corrupted = manifest.clone();
    let clean = corrupted
        .entries
        .iter()
        .position(|entry| !baseline.iter().any(|finding| finding.path() == entry.path))
        .expect("the manifest cannot be wrong about every one of its own entries");
    corrupted.entries[clean].digest = "0".repeat(64);

    let after = source_manifest::findings(&corrupted, &index);
    assert_eq!(
        after.len(),
        baseline.len() + 1,
        "corrupting one digest must add one finding and no more: before {baseline:?}, after {after:?}"
    );
    assert!(
        after
            .iter()
            .any(|finding| finding.path() == manifest.entries[clean].path),
        "the new finding has to name the path that was corrupted: {after:?}"
    );
}

#[test]
fn a_listed_path_the_index_does_not_hold_is_reported() {
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    let paths = listed_paths(&manifest);
    let index = index_of(&root, &paths);

    let mut extra = manifest.clone();
    extra.entries.push(Entry {
        digest: "0".repeat(64),
        path: "scripts/this-file-was-never-in-the-index.ps1".to_owned(),
    });

    let found = source_manifest::findings(&extra, &index);
    assert!(
        found.iter().any(
            |finding| matches!(finding, source_manifest::Finding::NotInTheIndex { path }
                if path == "scripts/this-file-was-never-in-the-index.ps1")
        ),
        "a listed path the index does not hold is a fact about the manifest, not something to \
         pass over: {found:?}"
    );
}

#[test]
fn a_path_that_leaves_the_repository_is_refused_rather_than_unhashed() {
    // The index lists the paths it lists; a manifest naming something outside
    // the repository must be reported, not silently skipped, because skipping
    // it would leave the manifest looking shorter and still green.
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    let paths = listed_paths(&manifest);
    let index = index_of(&root, &paths);

    for outside in ["../outside.txt", "/etc/passwd", "Cargo.toml/../Cargo.toml"] {
        let mut manifest = manifest.clone();
        manifest.entries.push(Entry {
            digest: "0".repeat(64),
            path: outside.to_owned(),
        });
        let found = source_manifest::findings(&manifest, &index);
        assert!(
            found.iter().any(
                |finding| matches!(finding, source_manifest::Finding::NotInTheIndex { path }
                    if path == outside)
            ),
            "{outside} must be reported: {found:?}"
        );
    }
}

// --- the manifest as a file ---------------------------------------------

#[test]
fn the_manifest_does_not_list_itself() {
    // Not an oversight and not fixable: a file cannot record its own digest,
    // because writing the digest changes the content the digest is of. What
    // covers the manifest's own bytes is git — the object the commit stores is
    // what a clone materialises, and `git fsck` is what verifies it. So the
    // exclusion is deliberate, and this test exists so that a contributor who
    // tries to close the gap meets the reason rather than the regress.
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    assert!(
        !manifest.entries.iter().any(|entry| entry.path == MANIFEST),
        "{MANIFEST} lists itself, which cannot be satisfied: the digest would have to be of a file \
         containing that digest. Its own bytes are covered by the commit that carries it."
    );
}

#[test]
fn the_listed_set_is_not_empty_and_names_each_path_once() {
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");

    // Named rather than counted, so this says what the manifest is *for* rather
    // than how big it happens to be. The last three are the files every
    // acceptance commit moves, which is why the regeneration step in
    // `CONTRIBUTING.md` is not optional on the commit that records an
    // acceptance.
    for expected in [
        "Cargo.toml",
        ".gitattributes",
        "progress/HANDOFF.md",
        "progress/state.json",
        "tasks/tasks.json",
    ] {
        assert!(
            manifest.entries.iter().any(|entry| entry.path == expected),
            "{expected} is not listed, so this manifest no longer covers what it is for"
        );
    }

    let mut seen = std::collections::BTreeSet::new();
    for entry in &manifest.entries {
        assert!(
            seen.insert(entry.path.clone()),
            "{} is listed twice, so one of its two digests is the only one that is checked and a \
             reader cannot tell which",
            entry.path
        );
    }
}

#[test]
fn every_listed_path_is_relative_and_inside_the_repository() {
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    for entry in &manifest.entries {
        assert!(
            !entry.path.starts_with('/')
                && !entry.path.contains(":\\")
                && !entry.path.contains('\\'),
            "{} is not a repository-relative, forward-slashed path; the manifest is read on three \
             platforms and a Windows spelling would not resolve on the other two",
            entry.path
        );
        assert!(
            !entry
                .path
                .split('/')
                .any(|part| part == ".." || part == "."),
            "{} contains a `.` or `..` segment",
            entry.path
        );
        assert!(
            !entry.path.starts_with("target/"),
            "{} is under target/, which is not committed, so a clone would report it as absent",
            entry.path
        );
    }
}

#[test]
fn reading_the_manifest_and_writing_it_back_gives_the_same_bytes() {
    // The regeneration tool rewrites this file, so a round trip that did not
    // preserve it would show up as a diff on every regeneration rather than as
    // a change someone made.
    let root = sure_testkit::repository_root();
    let text =
        std::fs::read_to_string(root.join(MANIFEST)).expect("the manifest is in the checkout");
    let manifest = source_manifest::parse(&text).expect("the committed manifest must parse");
    assert_eq!(
        source_manifest::render(&manifest),
        text,
        "parsing and writing back changed {MANIFEST}"
    );
    assert!(
        manifest.trailing_newline == text.ends_with('\n'),
        "the trailing newline state was not preserved"
    );
}

#[test]
fn a_correction_changes_the_digests_and_nothing_else() {
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).expect("the manifest must be readable");
    let paths = listed_paths(&manifest);
    let index = index_of(&root, &paths);

    let corrected = source_manifest::corrected(&manifest, &index);
    assert_eq!(
        corrected.entries.len(),
        manifest.entries.len(),
        "a correction must not add or remove a path: what belongs in a curated selection is not a \
         program's call"
    );
    for (before, after) in manifest.entries.iter().zip(&corrected.entries) {
        assert_eq!(before.path, after.path, "a correction reordered the file");
    }
    assert_eq!(corrected.trailing_newline, manifest.trailing_newline);

    // The corrected manifest is the one the check accepts, which is what makes
    // "run the tool" the whole of the repair.
    assert!(
        source_manifest::findings(&corrected, &index).is_empty(),
        "correcting the manifest did not make it agree with the index"
    );
}

// --- the ways a checker comes back empty --------------------------------

#[test]
fn a_manifest_with_no_entries_is_not_a_pass() {
    assert!(
        source_manifest::parse("").is_err(),
        "an empty manifest verifies nothing and must be refused rather than reported as clean"
    );
    assert!(
        source_manifest::parse("\n\n").is_err(),
        "a manifest of blank lines verifies nothing"
    );
}

#[test]
fn a_line_that_is_not_an_entry_is_refused() {
    for text in [
        "not a manifest at all\n",
        "688121b69603ddb2abb3ce60eddea3d2230a5fc16e38cff1f684c3436fa90caf Cargo.toml\n",
        "688121b69603ddb2abb3ce60eddea3d2230a5fc16e38cff1f684c3436fa90caf  \n",
        // Uppercase is refused rather than folded: two spellings of one digest
        // compare unequal as text, and everything here is text comparison.
        "688121B69603DDB2ABB3CE60EDDEA3D2230A5FC16E38CFF1F684C3436FA90CAF  Cargo.toml\n",
        "688121b69603ddb2abb3ce60eddea3d2230a5fc16e38cff1f684c3436fa90ca  Cargo.toml\n",
    ] {
        assert!(
            source_manifest::parse(text).is_err(),
            "this must be refused rather than read as an entry: {text:?}"
        );
    }
}

#[test]
fn a_manifest_that_is_not_there_is_an_error_rather_than_an_empty_result() {
    // A missing manifest is a broken checkout, not a finding about its
    // contents, and the two must not arrive as the same answer.
    let missing = Path::new("target/tmp/p15t020-this-directory-does-not-exist");
    let error = source_manifest::read(missing).expect_err("there is no manifest there");
    assert!(
        error.to_string().contains(MANIFEST),
        "the message must name the file it could not read: {error}"
    );
}

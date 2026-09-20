//! Asking the volume, not the operating system, whether two spellings of a path
//! name one file.
//!
//! [`super::compare`] answers *are these the same location* from the path text
//! alone — "Nothing here consults the filesystem" — and that is right for the
//! question it is asked, because its callers need an answer about a store that
//! does not exist yet. It is wrong for [`crate::recheck_lifecycle`]'s question.
//! Whether `src/EMAIL/SEND.rs` and `src/email/send.rs` are the same finding is a
//! fact about the volume the project sits on, and the operating system's name is
//! not it.
//!
//! # Why the platform's name is not the answer
//!
//! CI run `35544579833`, one workflow, one probe, two jobs:
//!
//! - `rust (macos-latest)`, job `106168109227`: wrote `src/EMAIL/SEND.rs`, read
//!   `src/email/send.rs`, got those bytes back, and `read_dir` reported one
//!   entry — **one file**;
//! - `rust (ubuntu-latest)`, job `106168109213`: the same probe, `NotFound` and
//!   two entries — **two files**.
//!
//! Two `#[cfg(not(windows))]` platforms, opposite answers, one run. The reading
//! is written up with the job logs beside it in
//! `target/tmp/p15t018-probe/reading.md`. So a `cfg!` on the platform — and a
//! `cfg(target_os = "macos")` as much as the `cfg(windows)` it would replace —
//! answers a question about the operating system when the question is about the
//! volume, and macOS ships case-folding volumes and case-keeping ones.
//!
//! # What it asks
//!
//! The volume that holds `directory`, in three signed answers:
//!
//! 1. an entry's name is taken from `directory`'s own listing and its case is
//!    flipped — `Send.rs` becomes `sEND.RS`;
//! 2. if the flipped spelling is **also an entry of this listing**, the volume
//!    holds two names that differ only in case, which a volume that folds case
//!    cannot do: it keeps case.
//! 3. otherwise the volume is asked whether that spelling resolves, and the
//!    answer is signed:
//!    - it resolves (`Ok`), and the listing does not hold it — the volume
//!      resolved a name it does not hold: it folds case;
//!    - it does not resolve (`NotFound`) — the volume keeps case;
//!    - anything else — the question could not be put.
//!
//! The flipped spelling is a child of `directory`, so it is on `directory`'s own
//! volume by construction and no mount point is crossed to ask. It **reads**: it
//! writes no file, creates no directory, and deletes nothing, which is why it can
//! be aimed at a project SURE is checking without touching it.
//!
//! The lookup is `symlink_metadata` rather than `metadata` on purpose: the
//! question is whether the *name* resolves, and following a symlink would report
//! a broken link as an absent name.
//!
//! # What it does when it cannot tell
//!
//! Returns `None` — a directory that cannot be listed, an entry that cannot be
//! read (a partial listing cannot support *this spelling is not held here*, so
//! it is not used at all), a directory where nothing has a case to flip, a
//! lookup that failed for a reason other than `NotFound`.
//!
//! ## Which way it errs when it cannot tell
//!
//! [`case_rule_of_volume_or_sensitive`] answers [`CaseSensitivity::Sensitive`],
//! and that is the direction to err in, because the two mistakes are not
//! symmetric:
//!
//! - answering [`CaseSensitivity::Insensitive`] about a volume that keeps case
//!   **merges two files into one identity**. In [`crate::recheck_lifecycle`] a
//!   previous finding and a current finding about two different files get one
//!   [`FindingKey`](crate::recheck_lifecycle::FindingKey): `reconcile` reports
//!   this run's spelling and drops the previous one from the verdict, and
//!   `previous_open_findings` drops a stored open finding as a duplicate of the
//!   other. A problem that exists stops being reported — a false green, and the
//!   one thing `CLAUDE.md` says is more serious than a visible error;
//! - answering [`CaseSensitivity::Sensitive`] about a volume that folds means
//!   two spellings of one file are two identities: the verdict carries two
//!   findings for one problem, and a previous finding is carried open beside
//!   this run's copy of it. That is duplicate material a reader can see.
//!
//! SURE reports twice rather than merging wrongly.
//!
//! # What it costs, and what it cannot see
//!
//! One directory listing and at most one lookup, so a caller asks once per run
//! and holds the answer ([`crate::recheck_lifecycle::case_rule_for`]) rather
//! than once per path. The listing and the lookup are two calls, so a name that
//! appears between them is read as folding; that is the class of race every
//! filesystem check carries, and its consequence is the merging direction above
//! rather than the visible one. A mount point *below* `directory` can hold paths
//! whose case rule is another volume's; this answers about the volume
//! `directory` itself is on.

use std::io::ErrorKind;
use std::path::Path;

use super::compare::CaseSensitivity;

/// What a volume said about one spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// A name spelled this way resolves to an entry.
    Resolves,
    /// The volume holds no entry by this name.
    Absent,
    /// The question could not be put.
    Unasked,
}

/// The case rule of the volume that holds `directory`, or `None` when that
/// volume could not be asked.
///
/// The three answers, what `None` means, and which way a caller should err when
/// it gets `None` are the module documentation's subject. Most callers want
/// [`case_rule_of_volume_or_sensitive`], which has decided the last of those.
#[must_use]
pub fn case_rule_of_volume(directory: &Path) -> Option<CaseSensitivity> {
    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(directory).ok()? {
        // A listing with a hole in it is not a listing: step 2 of the probe
        // concludes "the listing does not hold this spelling", which a missing
        // line would make false.
        let entry = entry.ok()?;
        // A name that is not Unicode cannot be the flipped spelling of one that
        // is, so leaving it out of the listing can only cost a candidate.
        if let Ok(name) = entry.file_name().into_string() {
            names.push(name);
        }
    }
    rule_from_listing(&names, |flipped| {
        match std::fs::symlink_metadata(directory.join(flipped)) {
            Ok(_) => Answer::Resolves,
            Err(error) if error.kind() == ErrorKind::NotFound => Answer::Absent,
            Err(_) => Answer::Unasked,
        }
    })
}

/// [`case_rule_of_volume`], with the answer `None` is replaced by.
///
/// When the volume cannot be asked the rule is [`CaseSensitivity::Sensitive`],
/// for the reason the module documentation gives at length: the other answer
/// merges two files into one identity and takes a finding out of the verdict,
/// and this one reports a problem twice.
#[must_use]
pub fn case_rule_of_volume_or_sensitive(directory: &Path) -> CaseSensitivity {
    case_rule_of_volume(directory).unwrap_or(CaseSensitivity::Sensitive)
}

/// The probe's decision, over a listing and a way to put one spelling to the
/// volume.
///
/// Split from the two filesystem calls so that every branch — including the two
/// only a case-keeping volume can produce — can be pinned on one machine.
fn rule_from_listing(
    names: &[String],
    resolves: impl Fn(&str) -> Answer,
) -> Option<CaseSensitivity> {
    for name in names {
        let Some(flipped) = flipped_case(name) else {
            continue;
        };
        if names.iter().any(|held| held == &flipped) {
            // Both spellings are entries of this one directory, which a volume
            // that folds case cannot hold at once.
            return Some(CaseSensitivity::Sensitive);
        }
        return match resolves(&flipped) {
            Answer::Resolves => Some(CaseSensitivity::Insensitive),
            Answer::Absent => Some(CaseSensitivity::Sensitive),
            Answer::Unasked => None,
        };
    }
    // Nothing here has a case to flip, so there is no question to put.
    None
}

/// `name` with the case of every character flipped, or `None` when it has no
/// character whose case can be flipped on its own.
///
/// One character at a time, and only where the counterpart is a single
/// character: `ß` uppercases to `SS`, which is a longer name rather than a
/// differently-cased one, and asking the volume about *that* would be asking
/// about a different file. `changed` is true only when a character was actually
/// mapped, so the result is always a different string from the input.
fn flipped_case(name: &str) -> Option<String> {
    let mut flipped = String::with_capacity(name.len());
    let mut changed = false;
    for character in name.chars() {
        match other_case(character) {
            Some(other) => {
                flipped.push(other);
                changed = true;
            }
            None => flipped.push(character),
        }
    }
    changed.then_some(flipped)
}

/// `character`'s counterpart in the other case, when it has exactly one and it
/// is not the character itself.
fn other_case(character: char) -> Option<char> {
    if character.is_lowercase() {
        only_one(character.to_uppercase(), character)
    } else if character.is_uppercase() {
        only_one(character.to_lowercase(), character)
    } else {
        None
    }
}

/// The one character `mapped` yields, when it yields exactly one and it is not
/// `character` itself.
fn only_one(mut mapped: impl Iterator<Item = char>, character: char) -> Option<char> {
    let first = mapped.next()?;
    if mapped.next().is_some() || first == character {
        None
    } else {
        Some(first)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A directory under `target/tmp`, git-ignored and on the same volume as the
    /// checkout — the volume a probe's answer is about. The name carries the
    /// process id for the reason `store/mod.rs` records in full: freshness must
    /// not depend on a deletion succeeding.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = crate::store::scratch_root().join(format!("{name}-{}", std::process::id()));
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear {}: {error}", dir.display()),
        }
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn names(listing: &[&str]) -> Vec<String> {
        listing.iter().map(|name| (*name).to_owned()).collect()
    }

    // The four branches of the decision, driven directly so that all four are
    // pinned on every platform: two of them need a case-keeping volume to occur
    // on a real one, and a machine whose volume folds case could otherwise never
    // run them.

    #[test]
    fn a_spelling_the_listing_does_not_hold_that_resolves_is_a_volume_that_folds() {
        let listing = names(&["Send.rs"]);
        let rule = rule_from_listing(&listing, |asked| {
            assert_eq!(asked, "sEND.RS", "the probe asked about another name");
            Answer::Resolves
        });
        assert_eq!(rule, Some(CaseSensitivity::Insensitive));
    }

    #[test]
    fn a_spelling_the_listing_does_not_hold_that_does_not_resolve_keeps_case() {
        let listing = names(&["Send.rs"]);
        let rule = rule_from_listing(&listing, |_| Answer::Absent);
        assert_eq!(rule, Some(CaseSensitivity::Sensitive));
    }

    #[test]
    fn a_listing_holding_both_spellings_is_a_volume_that_keeps_case() {
        // The answer that matters most and is hardest to reach: where the
        // volume holds `Send.rs` *and* `sEND.RS`, the flipped spelling resolves,
        // and a probe without this branch would read that as folding. The
        // lookup must not even be put: the listing has answered.
        let listing = names(&["Send.rs", "sEND.RS"]);
        let rule = rule_from_listing(&listing, |_| {
            panic!("the volume was asked a question its own listing had answered")
        });
        assert_eq!(rule, Some(CaseSensitivity::Sensitive));
    }

    #[test]
    fn a_lookup_that_could_not_be_put_is_not_answered() {
        let listing = names(&["Send.rs"]);
        assert_eq!(rule_from_listing(&listing, |_| Answer::Unasked), None);
    }

    #[test]
    fn a_listing_with_nothing_to_flip_cannot_be_asked() {
        // These entries have no cased character: there is no second spelling of
        // either name to put to the volume.
        let listing = names(&["2024", "42", "配置"]);
        assert_eq!(
            rule_from_listing(&listing, |_| panic!("nothing was asked")),
            None
        );
    }

    #[test]
    fn a_listing_of_nothing_at_all_cannot_be_asked() {
        assert_eq!(rule_from_listing(&[], |_| Answer::Resolves), None);
    }

    #[test]
    fn the_first_name_with_a_case_to_flip_is_the_one_asked_about() {
        let listing = names(&["2024", "Send.rs", "other.rs"]);
        let rule = rule_from_listing(&listing, |asked| {
            assert_eq!(asked, "sEND.RS");
            Answer::Absent
        });
        assert_eq!(rule, Some(CaseSensitivity::Sensitive));
    }

    #[test]
    fn a_name_whose_case_cannot_be_flipped_on_its_own_is_not_a_candidate() {
        // `ß` uppercases to `SS`: two characters, so the result would be a
        // longer name and not a second spelling of this one. `a` can be flipped,
        // so `maß` → `MAß` still differs from the input by case alone.
        assert_eq!(flipped_case("maß"), Some("MAß".to_owned()));
        assert_eq!(flipped_case("ß"), None);
        assert_eq!(flipped_case("2024"), None);
        assert_eq!(flipped_case(""), None);
        // Non-ASCII cased characters are as flippable as ASCII ones.
        assert_eq!(flipped_case("Straße"), Some("sTRAßE".to_owned()));
    }

    /// The probe against a directory on the volume this suite runs on, checked
    /// against the answer the bytes give.
    ///
    /// The measurement is the one CI run `35544579833` used: write bytes to one
    /// spelling, read them back from the other. That is the volume's own answer,
    /// so this test asserts the probe agrees with it wherever it runs and needs
    /// no platform gate — which is the point of asking the volume rather than
    /// the operating system.
    ///
    /// **What it does if the probe cannot answer**: it fails rather than falling
    /// back to the rule a caller would use. A test that accepted the fallback
    /// would pass while asking nothing, which is the defect the comparison
    /// exists to catch.
    #[test]
    fn the_probe_agrees_with_what_the_bytes_do() {
        let dir = scratch("volume-probe-truth");
        std::fs::write(dir.join("Send.rs"), b"the-bytes\n").expect("a file to read back");

        let measured = match std::fs::read(dir.join("sEND.RS")) {
            Ok(bytes) if bytes == b"the-bytes\n" => CaseSensitivity::Insensitive,
            Ok(other) => panic!(
                "the volume resolved the other spelling to different bytes ({other:?}), so it \
                 holds two files and neither answer describes it"
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => CaseSensitivity::Sensitive,
            Err(error) => panic!("the read-back could not be made: {error}"),
        };

        let probed = case_rule_of_volume(&dir).unwrap_or_else(|| {
            panic!(
                "the probe could not ask the volume at {}, so this test would have compared \
                 nothing with nothing",
                dir.display()
            )
        });
        assert_eq!(
            probed,
            measured,
            "the probe answered {probed:?} and the bytes say {measured:?} about {}",
            dir.display()
        );
    }

    /// This machine's volume, named. The assertion differs by platform class
    /// because the *answer* differs, and it says what it is asking the platform
    /// rather than the operating system: what the volume the checkout sits on
    /// does with two spellings of one name. `CaseSensitivity::platform` states
    /// the same convention for the same two classes of platform, and the test
    /// above is the one that holds the probe to the bytes.
    ///
    /// A machine that mounts a case-keeping volume at the checkout fails this on
    /// purpose: the convention would be wrong about it, and the failure message
    /// names what the volume said.
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn the_default_volume_here_folds_case() {
        let dir = scratch("volume-probe-default");
        std::fs::write(dir.join("Send.rs"), b"x").expect("a file to flip the case of");
        let rule = case_rule_of_volume(&dir);
        assert_eq!(
            rule,
            Some(CaseSensitivity::Insensitive),
            "the volume holding this checkout answered {rule:?}"
        );
    }

    /// The other half of the pair above, on the platforms whose shipped volumes
    /// keep case. `#[cfg(not(windows))]` would have put this assertion on macOS
    /// as well, where CI run `35544579833` measured the opposite answer.
    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn the_default_volume_here_keeps_case() {
        let dir = scratch("volume-probe-default");
        std::fs::write(dir.join("Send.rs"), b"x").expect("a file to flip the case of");
        let rule = case_rule_of_volume(&dir);
        assert_eq!(
            rule,
            Some(CaseSensitivity::Sensitive),
            "the volume holding this checkout answered {rule:?}"
        );
    }

    /// The shape only a case-keeping volume can hold, built rather than
    /// simulated: two entries whose names differ only in case.
    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn a_directory_holding_both_spellings_is_reported_as_keeping_case() {
        let dir = scratch("volume-probe-both");
        std::fs::write(dir.join("Send.rs"), b"one").expect("the first spelling");
        std::fs::write(dir.join("sEND.RS"), b"other").expect("the second spelling");
        assert_eq!(
            std::fs::read_dir(&dir).expect("the listing").count(),
            2,
            "the two spellings are not two entries here, so this volume cannot hold the shape \
             the test is about"
        );
        assert_eq!(
            case_rule_of_volume(&dir),
            Some(CaseSensitivity::Sensitive),
            "a volume holding both spellings at once does not fold case"
        );
    }

    #[test]
    fn a_directory_that_is_not_there_cannot_be_asked() {
        let missing = crate::store::scratch_root().join("volume-probe-absent");
        let _ = std::fs::remove_dir_all(&missing);
        assert_eq!(case_rule_of_volume(&missing), None);
        // And the fallback a caller gets: two spellings are two files, so a
        // finding is reported twice rather than two problems merged into one.
        assert_eq!(
            case_rule_of_volume_or_sensitive(&missing),
            CaseSensitivity::Sensitive
        );
    }

    #[test]
    fn an_empty_directory_cannot_be_asked() {
        let dir = scratch("volume-probe-empty");
        assert_eq!(case_rule_of_volume(&dir), None);
        assert_eq!(
            case_rule_of_volume_or_sensitive(&dir),
            CaseSensitivity::Sensitive
        );
    }

    #[test]
    fn a_probe_writes_nothing_into_the_directory_it_asks_about() {
        // The reason this can be aimed at a project SURE is checking: the
        // question is put with two reads, and a project's own watcher sees no
        // file appear and no file disappear.
        let dir = scratch("volume-probe-readonly");
        std::fs::write(dir.join("Send.rs"), b"x").expect("a file to flip the case of");
        let before = listing(&dir);
        let _ = case_rule_of_volume(&dir);
        assert_eq!(before, listing(&dir));
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut found: Vec<String> = std::fs::read_dir(dir)
            .expect("the listing")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        found.sort();
        found
    }
}

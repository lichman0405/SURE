//! Reading what `git status --porcelain=v2` said.
//!
//! Git has two status formats for programs. Version 1 is `XY path`, which pads,
//! quotes and renames its fields depending on configuration. Version 2 is
//! documented as the stable one for tools: a record per change, fields in a
//! fixed order, and — with `-z` — no quoting at all, so a path containing a
//! space, a quote, a backslash or a newline arrives as itself.
//!
//! That last part is why this module works in **bytes**: the paths are not
//! necessarily text, and text is the thing that loses them. See
//! [`path_from_git_bytes`].
//!
//! # Records this module refuses
//!
//! An unrecognised record is an error, not something skipped. Every record is a
//! change, so dropping one produces a fingerprint that omits a change — the
//! value looks complete, evidence about the old state compares equal to it, and
//! an old result is reported as current. A visible refusal is the better
//! failure, and it is the one a person can act on.
//!
//! Two records are deliberately unreachable and still refused:
//!
//! - `2`, a rename or copy. SURE asks Git not to detect renames
//!   ([`super::Git::STATUS_ARGUMENTS`]), because detection is a similarity
//!   heuristic whose threshold is configuration and whose result can differ
//!   between Git versions — and a fingerprint that depends on a heuristic is a
//!   fingerprint that changes when nothing did. With renames off, the same state
//!   arrives as a deletion and an addition, both of which are read here. A `2`
//!   arriving anyway means Git is not doing what SURE asked, and the honest
//!   response to that is to stop rather than to guess.
//! - `!`, an ignored path. SURE does not ask for ignored paths, and it would not
//!   want them if it had them: an ignored path is one the project has already
//!   said is not part of it.
//!
//! Headers (`# branch.oid` and friends) are different, and unknown ones are
//! ignored. A header describes the branch, not a file, and Git adding one is not
//! a change SURE would leave out of a fingerprint.

use std::path::PathBuf;

use super::super::error::FingerprintError;

/// Which of Git's records a path came from.
///
/// The distinction matters in the fingerprint rather than in the reading: a
/// tracked change and an untracked file are recorded separately, so a caller can
/// tell "this project is dirty" from "this project has files Git has not been
/// told about".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordKind {
    /// A path in the repository whose content differs from HEAD, or which HEAD
    /// does not have.
    Tracked,
    /// A path in the working tree that the repository neither tracks nor
    /// ignores.
    Untracked,
    /// A path with a merge conflict in it: Git holds more than one version and
    /// the working tree holds the markers.
    Unmerged,
}

impl RecordKind {
    /// The tag this kind contributes to the digest.
    ///
    /// Written into the hash so that the same path cannot be recorded as one
    /// kind in one run and another kind in the next and produce the same
    /// fingerprint.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Tracked => "tracked",
            Self::Untracked => "untracked",
            Self::Unmerged => "unmerged",
        }
    }

    /// Whether this is a change to a file the repository tracks.
    pub(super) const fn is_tracked(self) -> bool {
        match self {
            Self::Tracked | Self::Unmerged => true,
            Self::Untracked => false,
        }
    }
}

/// One path Git reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Record {
    /// The path relative to the repository root, as the operating system will
    /// accept it.
    pub(super) path: PathBuf,
    /// The same path as the bytes Git printed.
    ///
    /// Carried alongside rather than derived from [`Self::path`] because the
    /// digest is over what Git said, and a conversion that loses a byte would
    /// otherwise make two different names hash the same.
    pub(super) bytes: Vec<u8>,
    /// Which part of the status it came from.
    pub(super) kind: RecordKind,
    /// Whether Git's mode fields say this is a submodule's working tree.
    ///
    /// A submodule is a directory as far as the filesystem is concerned, and the
    /// ignore tables are two tables — one for names that are never a directory a
    /// project keeps its work in, one for names that are never a file. Reading
    /// it as a file would ask the wrong table. A deleted submodule has no
    /// filesystem entry left to ask, which is why this comes from Git's mode
    /// fields and not from a stat.
    pub(super) gitlink: bool,
}

/// What `git status --porcelain=v2 --branch` said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Status {
    /// The commit that is checked out, or `(initial)` in a repository that has
    /// no commits yet, or `None` when Git did not say.
    pub(super) head: Option<String>,
    /// The branch that is checked out, or `None` when HEAD is detached.
    pub(super) branch: Option<String>,
    /// Every path Git reported, in the order it reported them.
    pub(super) records: Vec<Record>,
}

impl Status {
    /// The commit that is checked out.
    ///
    /// # Errors
    ///
    /// [`FingerprintError::MissingHead`] when Git's output had no
    /// `# branch.oid` line. Git emits one whenever `--branch` is passed, so this
    /// means SURE is not reading what it thinks it is reading, and a fingerprint
    /// over the working tree alone would call a checkout of a different commit
    /// the same project state.
    pub(super) fn head(&self) -> Result<&str, FingerprintError> {
        self.head.as_deref().ok_or(FingerprintError::MissingHead)
    }
}

/// Read `git status --porcelain=v2 --branch -z` output.
///
/// # Errors
///
/// Returns [`FingerprintError::UnrecognizedStatus`] for a record this module
/// does not understand, including one whose path it cannot turn into a path the
/// operating system would accept. See the module comment for why refusing beats
/// skipping.
pub(super) fn parse(output: &[u8]) -> Result<Status, FingerprintError> {
    let mut status = Status {
        head: None,
        branch: None,
        records: Vec::new(),
    };

    // `-z` terminates every record with a NUL, including the last one, so the
    // split ends with an empty slice that is not a record.
    for record in output.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        match record.first() {
            Some(b'#') => read_header(record, &mut status),
            Some(b'1') => status
                .records
                .push(read_change(record, 8, RecordKind::Tracked)?),
            Some(b'u') => status
                .records
                .push(read_change(record, 10, RecordKind::Unmerged)?),
            Some(b'?') => status.records.push(read_untracked(record)?),
            _ => return Err(unrecognized(record)),
        }
    }

    Ok(status)
}

/// Read a `# ` header line, ignoring the ones SURE has no use for.
fn read_header(record: &[u8], status: &mut Status) {
    let Some(rest) = record.strip_prefix(b"# ") else {
        // Not a shape Git emits. Ignored rather than refused for the reason in
        // the module comment: a header is not a file, so nothing can be missed
        // by not reading it.
        return;
    };
    if let Some(value) = rest.strip_prefix(b"branch.oid ") {
        status.head = Some(String::from_utf8_lossy(value).into_owned());
    } else if let Some(value) = rest.strip_prefix(b"branch.head ") {
        // Git's own spelling for a detached HEAD. Not a branch name, and
        // recording it as one would be SURE inventing a branch called
        // `(detached)` that nobody can check out.
        status.branch = match value {
            b"(detached)" => None,
            name => Some(String::from_utf8_lossy(name).into_owned()),
        };
    }
}

/// Read a `1` or `u` record, whose path is the field after `count` others.
fn read_change(record: &[u8], count: usize, kind: RecordKind) -> Result<Record, FingerprintError> {
    let Some(path) = field_after(record, count) else {
        return Err(unrecognized(record));
    };
    let (path, bytes, directory) = split_path(path)?;
    Ok(Record {
        path,
        bytes,
        kind,
        gitlink: directory || is_gitlink(record),
    })
}

/// Read a `?` record, whose path is the field after the marker.
fn read_untracked(record: &[u8]) -> Result<Record, FingerprintError> {
    let Some(path) = field_after(record, 1) else {
        return Err(unrecognized(record));
    };
    let (path, bytes, directory) = split_path(path)?;
    Ok(Record {
        path,
        bytes,
        kind: RecordKind::Untracked,
        // Git reports an untracked directory as one record ending in `/`, so a
        // repository cloned into the project without being added arrives here
        // and not as a `1`.
        gitlink: directory,
    })
}

/// The bytes after the first `count` space-separated fields.
///
/// `None` when the record has fewer fields than that, which is how a truncated
/// or future record is caught. The path is everything after the last field, so a
/// path containing a space — or *starting* with one — arrives whole: splitting
/// with a maximum of `count + 1` parts leaves the remainder untouched.
fn field_after(record: &[u8], count: usize) -> Option<&[u8]> {
    let mut fields = record.splitn(count + 1, |byte| *byte == b' ');
    for _ in 0..count {
        fields.next()?;
    }
    fields.next()
}

/// Whether any of Git's mode fields says this entry is a submodule.
///
/// The mode fields are within the first seven for both the `1` and the `u`
/// record, and a file mode is never `160000`, so one scan answers it for either.
fn is_gitlink(record: &[u8]) -> bool {
    record
        .split(|byte| *byte == b' ')
        .take(7)
        .any(|field| field == b"160000")
}

/// One path, as the operating system's kind of path, as Git's bytes, and
/// whether Git marked it as a directory.
///
/// Git marks an untracked directory with a trailing `/`. Removing it is either
/// the right thing — for that record — or a no-op, because no other record has
/// one: a file cannot be named with a separator on any platform SURE runs on.
fn split_path(bytes: &[u8]) -> Result<(PathBuf, Vec<u8>, bool), FingerprintError> {
    let directory = bytes.ends_with(b"/");
    let trimmed = bytes.strip_suffix(b"/").unwrap_or(bytes);
    Ok((path_from_git_bytes(trimmed)?, trimmed.to_vec(), directory))
}

/// A path Git wrote, as the operating system's own kind of path.
///
/// Git writes paths as bytes. Unix file names are bytes, so on Unix this is the
/// path itself and nothing is lost. Windows file names are UTF-16, so the bytes
/// have to become text — and a byte sequence that is not UTF-8 has no Windows
/// name to become. Returning an error for one rather than the replacement
/// characters `from_utf8_lossy` would give is the difference between refusing
/// and quietly reading a *different* file, or finding none where one exists.
///
/// What this does **not** resolve is a name Git itself could not represent: Git
/// for Windows stores the index as UTF-8, so a Windows name that is not UTF-8
/// may already have been replaced by the time SURE sees it. See
/// `docs/architecture/FINGERPRINTING.md`, which records that as a known gap
/// rather than pretending this function closes it.
#[cfg(unix)]
pub(super) fn path_from_git_bytes(bytes: &[u8]) -> Result<PathBuf, FingerprintError> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// See the Unix definition above; this is the same rule for a platform whose
/// file names are not bytes.
#[cfg(not(unix))]
pub(super) fn path_from_git_bytes(bytes: &[u8]) -> Result<PathBuf, FingerprintError> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(PathBuf::from(text)),
        Err(_) => Err(FingerprintError::UnrecognizedStatus {
            record: format!(
                "a path that is not valid Unicode: {}",
                String::from_utf8_lossy(bytes)
            ),
        }),
    }
}

/// A record, as text for an error message, cut short if it is very long.
///
/// Lossy on purpose: this is for a person to read, and the record itself is
/// already the thing that could not be understood.
pub(super) fn unrecognized(record: &[u8]) -> FingerprintError {
    const MOST: usize = 200;
    let text = String::from_utf8_lossy(record);
    let rendered = if text.chars().count() <= MOST {
        text.into_owned()
    } else {
        let mut cut: String = text.chars().take(MOST).collect();
        cut.push_str("...");
        cut
    };
    FingerprintError::UnrecognizedStatus { record: rendered }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Join records the way `-z` does, terminator included.
    fn output(lines: &[&[u8]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for line in lines {
            bytes.extend_from_slice(line);
            bytes.push(0);
        }
        bytes
    }

    const HEADER: &[u8] = b"# branch.oid 1111111111111111111111111111111111111111";
    const BRANCH: &[u8] = b"# branch.head main";

    #[test]
    fn an_ordinary_status_is_read_as_written() {
        let bytes = output(&[
            HEADER,
            BRANCH,
            b"# branch.upstream origin/main",
            b"# branch.ab +0 -0",
            b"1 .M N... 100644 100644 100644 1111111111111111111111111111111111111111 1111111111111111111111111111111111111111 a.txt",
            b"? new.txt",
        ]);
        let status = parse(&bytes).unwrap();

        assert_eq!(
            status.head().unwrap(),
            "1111111111111111111111111111111111111111"
        );
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.records.len(), 2);
        assert_eq!(status.records[0].path, PathBuf::from("a.txt"));
        assert_eq!(status.records[0].kind, RecordKind::Tracked);
        assert_eq!(status.records[1].path, PathBuf::from("new.txt"));
        assert_eq!(status.records[1].kind, RecordKind::Untracked);
    }

    #[test]
    fn a_path_that_starts_or_ends_with_a_space_arrives_whole() {
        // The reason `-z` is used at all, and the reason the path is taken as
        // "everything after the last field" rather than "the ninth field split
        // on spaces". A path with two spaces in it is the case a naive split
        // gets wrong, and the case a person creates by accident.
        let bytes = output(&[
            HEADER,
            b"1 .M N... 100644 100644 100644 1111111111111111111111111111111111111111 1111111111111111111111111111111111111111  leading and  trailing .txt",
            b"?  a name starting with a space",
        ]);
        let status = parse(&bytes).unwrap();

        assert_eq!(
            status.records[0].path,
            PathBuf::from(" leading and  trailing .txt")
        );
        assert_eq!(
            status.records[1].path,
            PathBuf::from(" a name starting with a space")
        );
        // And the bytes are the same string, not a re-derivation from it.
        assert_eq!(
            status.records[0].bytes,
            b" leading and  trailing .txt".to_vec()
        );
    }

    #[test]
    fn a_quoted_path_is_not_unquoted_because_it_never_was_quoted() {
        // `core.quotepath` makes version 1 status print `"a\\nb"` for a path
        // containing a newline. With `-z` there is no quoting, so the backslash
        // and the `n` are two ordinary characters — and a reader that
        // unescaped them would invent a file name nobody has.
        let bytes = output(&[HEADER, b"? a\\nb.txt", b"? \"quoted\".txt"]);
        let status = parse(&bytes).unwrap();
        assert_eq!(status.records[0].path, PathBuf::from("a\\nb.txt"));
        assert_eq!(status.records[1].path, PathBuf::from("\"quoted\".txt"));
    }

    #[test]
    fn an_untracked_directory_loses_gits_trailing_slash_and_keeps_the_fact() {
        // Git's marker that the record is a directory. Kept as `gitlink` — which
        // for an untracked record means "a whole tree, not a file" — because it
        // decides which of the two ignore tables the name is looked up in.
        let bytes = output(&[HEADER, b"? nested/"]);
        let status = parse(&bytes).unwrap();
        assert_eq!(status.records[0].path, PathBuf::from("nested"));
        assert_eq!(status.records[0].bytes, b"nested".to_vec());
        assert!(status.records[0].gitlink);
    }

    #[test]
    fn a_submodule_is_a_directory_whether_or_not_it_still_exists() {
        // The mode fields, and why they are read rather than a stat: a deleted
        // submodule has no filesystem entry left to ask, and asking the file
        // table about a directory name is how the ignore tables get applied to
        // the wrong kind of entry.
        let bytes = output(&[
            HEADER,
            b"1 .M S.M. 160000 160000 160000 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 sub",
            b"1 D. N... 100644 000000 000000 1111111111111111111111111111111111111111 0000000000000000000000000000000000000000 gone.txt",
        ]);
        let status = parse(&bytes).unwrap();
        assert!(status.records[0].gitlink);
        assert!(!status.records[1].gitlink);
    }

    #[test]
    fn an_unmerged_path_is_read_as_a_change() {
        // `u` records carry three modes and three hashes before the path, so a
        // reader that assumed the `1` layout would take a hash for a file name.
        let bytes = output(&[
            HEADER,
            b"u UU N... 100644 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 3333333333333333333333333333333333333333 conflicted.txt",
        ]);
        let status = parse(&bytes).unwrap();
        assert_eq!(status.records[0].path, PathBuf::from("conflicted.txt"));
        assert_eq!(status.records[0].kind, RecordKind::Unmerged);
        assert!(status.records[0].kind.is_tracked());
    }

    #[test]
    fn a_repository_with_no_commits_has_a_head_that_is_not_a_commit() {
        // Git's own spelling. Kept as the head rather than treated as missing:
        // a repository with no commits is a state, and an empty string would be
        // indistinguishable from Git having said nothing.
        let bytes = output(&[b"# branch.oid (initial)", BRANCH, b"? first.txt"]);
        let status = parse(&bytes).unwrap();
        assert_eq!(status.head().unwrap(), "(initial)");
        assert_eq!(status.branch.as_deref(), Some("main"));
    }

    #[test]
    fn a_detached_head_is_not_a_branch() {
        // `(detached)` is not a name anybody can check out, and recording it as
        // one would put a branch in the report that does not exist.
        let bytes = output(&[HEADER, b"# branch.head (detached)"]);
        let status = parse(&bytes).unwrap();
        assert_eq!(status.branch, None);
    }

    #[test]
    fn a_status_that_never_says_which_commit_is_an_error() {
        // Without HEAD there is no fingerprint: a checkout of another commit
        // with no local changes would share one with the state before it.
        let status = parse(&output(&[BRANCH, b"? new.txt"])).unwrap();
        assert_eq!(status.head, None);
        assert_eq!(status.head().unwrap_err(), FingerprintError::MissingHead);
    }

    #[test]
    fn an_unknown_header_is_ignored_and_an_unknown_record_is_not() {
        // The asymmetry, both ways. A header Git adds later describes the
        // branch; a record Git adds later is a change, and a fingerprint that
        // omits a change says "nothing moved" when something did.
        let bytes = output(&[HEADER, b"# branch.future something new", b"? new.txt"]);
        assert_eq!(parse(&bytes).unwrap().records.len(), 1);

        for record in [
            &b"2 R. N... 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 R100 new.txt"[..],
            &b"! ignored.txt"[..],
            &b"x something else entirely"[..],
        ] {
            let error = parse(&output(&[HEADER, record])).unwrap_err();
            assert!(
                matches!(error, FingerprintError::UnrecognizedStatus { .. }),
                "{:?} was accepted",
                String::from_utf8_lossy(record)
            );
        }
    }

    #[test]
    fn a_truncated_record_is_refused_rather_than_read_as_a_path() {
        // A `1` record missing its hash fields would otherwise yield a hash as
        // the file name, and the fingerprint would cover a path nobody has.
        for record in [
            &b"1 .M N... 100644 100644 100644 aaaa"[..],
            &b"1 .M"[..],
            &b"1"[..],
            &b"u UU N..."[..],
        ] {
            assert!(
                parse(&output(&[HEADER, record])).is_err(),
                "{:?} was accepted",
                String::from_utf8_lossy(record)
            );
        }
    }

    #[test]
    fn a_very_long_record_is_cut_short_in_the_message() {
        // The message is for a person. A record that arrived as 100 kB would be
        // reported as 100 kB, and the one part of it that says what went wrong
        // would be scrolled off the top.
        //
        // The record starts with `9`, which is not a record type SURE knows. A
        // long *path* is not this case and must not be treated as one: a path of
        // any length that a filesystem accepted is an ordinary path, and cutting
        // it short would be SURE editing the project's own history.
        let mut record = b"9 ".to_vec();
        record.extend(std::iter::repeat_n(b'x', 5_000));
        let error = parse(&output(&[HEADER, &record])).unwrap_err();
        let FingerprintError::UnrecognizedStatus { record } = error else {
            panic!("expected an unrecognized record");
        };
        assert!(record.len() < 300, "the record was not cut short");
        assert!(record.ends_with("..."), "{record}");
    }

    #[test]
    fn a_long_path_is_a_long_path_and_not_an_unreadable_record() {
        // The other half of the test above, and the reason it had to be written
        // twice. `--untracked-files=all` on a deep `node_modules`-less tree can
        // produce a path of several thousand characters, and refusing it as
        // "too long to understand" would refuse a real project.
        let mut record = b"? ".to_vec();
        record.extend(std::iter::repeat_n(b'x', 5_000));
        let status = parse(&output(&[HEADER, &record])).unwrap();
        assert_eq!(status.records.len(), 1);
        assert_eq!(status.records[0].path.as_os_str().len(), 5_000);
    }

    #[test]
    fn an_empty_status_is_a_clean_checkout_and_not_an_error() {
        // The shape a clean repository produces. It has to be distinguishable
        // from a failure, and it is: this parses, and the failure is an `Err`.
        let status = parse(&output(&[HEADER, BRANCH])).unwrap();
        assert!(status.records.is_empty());
        assert_eq!(status.head().unwrap().len(), 40);
    }

    #[test]
    fn output_with_no_trailing_nul_still_reads_its_last_record() {
        // `-z` terminates every record, so this cannot happen through SURE's own
        // invocation — but a reader that depended on the final NUL would drop
        // the last change if it ever did, which is the silent direction.
        let mut bytes = output(&[HEADER, b"? only.txt"]);
        bytes.pop();
        assert_eq!(parse(&bytes).unwrap().records.len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn a_path_that_is_not_valid_unicode_is_refused_on_windows() {
        // Windows file names are UTF-16, so there is no name for these bytes to
        // become, and `from_utf8_lossy` would give the replacement characters of
        // a *different* file.
        let error = path_from_git_bytes(&[0xED, 0xA0, 0x80]).unwrap_err();
        assert!(matches!(error, FingerprintError::UnrecognizedStatus { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_valid_unicode_is_the_path_on_unix() {
        use std::os::unix::ffi::OsStrExt;
        let bytes = [0xED, 0xA0, 0x80];
        let path = path_from_git_bytes(&bytes).unwrap();
        assert_eq!(path.as_os_str().as_bytes(), bytes);
    }
}

//! Reading a named file that a manifest might be, without ever confusing "not
//! there" with "there and SURE could not read it".
//!
//! # The distinction this module exists for
//!
//! Every caller in [`super::node`] asks the same question first — *is there a
//! `package.json` here?* — and there are four different answers:
//!
//! 1. there is nothing at that name;
//! 2. there is something, and it is a link, a directory or a pipe, which SURE
//!    does not read;
//! 3. there is a file, and SURE could not turn it into a value;
//! 4. there is a file, and here is what it says.
//!
//! Collapsing (1) and (3) into an `Option` is the mistake this module is built
//! to make impossible, and it is not a corner case. A project whose
//! `package.json` failed to parse would be reported as a project with no
//! manifest: SURE would say the project declares no scripts, no dependencies and
//! no package manager, and every one of those would be a claim about a file it
//! never read. That is a false statement about the project dressed as a negative
//! finding, which is the failure `CLAUDE.md` calls more serious than a visible
//! error.
//!
//! So there is no `Option` here. [`ReadFile`] has an arm per answer above, and
//! [`super::node::ManifestState`] carries all four through to the result.
//!
//! # Two rules taken from the rest of SURE rather than decided again
//!
//! **A link is never read through.** `crate::scan` does not follow one and
//! `crate::fingerprint` records one by its target; a manifest that
//! `read_to_string`'d a link would be the one place in the product that reads
//! whatever a project points at, including somewhere outside the project. A
//! manifest reached by a link is [`UnreadReason::NotReadableKind`].
//!
//! **A file is read within a stated limit, and reaching it is a refusal rather
//! than a partial read.** Half a `package.json` is not a smaller
//! `package.json`; it is a document that fails to parse, and it would fail with
//! a message about a syntax error the project does not have. The limit is
//! checked after reading one byte more than allowed, so a file that grows while
//! SURE reads it is caught too — the same rule `fingerprint::read` applies to a
//! hash, for the same reason.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

use super::DiscoverOptions;
use crate::paths::CaseSensitivity;
use crate::scan::{Scan, SkipReason};

/// Why a file that is there could not be turned into a value.
///
/// Not an error type. A file SURE could not read is a **finding about the
/// project**, so it travels inside the discovery result — see the module
/// comment of [`super`] for why it is not a [`crate::diagnostics`] entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnreadReason {
    /// Discovery had already read as many manifests as it will in one run.
    ///
    /// The count is of manifests, not bytes, and it exists because a workspace
    /// can name an unbounded number of members: without it, a `package.json`
    /// listing ten thousand paths would be ten thousand file reads per check.
    OutOfBudget {
        /// How many manifests one discovery will read.
        limit: usize,
    },
    /// Bigger than [`DiscoverOptions::max_manifest_bytes`].
    ///
    /// The size is not reported because it is not known: reading stops one byte
    /// past the limit, so any number here would be a floor presented as a
    /// measurement.
    TooLarge {
        /// The limit that was reached.
        limit: u64,
    },
    /// The operating system would not open it.
    Unreadable {
        /// The operating system's own words, carried as data.
        detail: String,
    },
    /// The bytes are not text SURE can read.
    ///
    /// A manifest is UTF-8 by every ecosystem's own specification, so this is a
    /// statement about the file rather than about SURE's encoding support.
    NotText {
        /// What the decoder said, carried as data.
        detail: String,
    },
    /// The text is not the format the file's name promises.
    NotParsed {
        /// What the parser said, carried as data.
        detail: String,
    },
    /// It parsed, and it is not the shape a manifest of this kind has.
    ///
    /// A `package.json` holding `[1, 2, 3]` or `"hello"` is valid JSON and is
    /// not a manifest. Reading fields out of it would produce a project with no
    /// scripts and no dependencies — the same false negative as an unparsed
    /// file, arriving by a different route.
    WrongShape {
        /// What shape it has instead, as one of this module's own phrases.
        found: &'static str,
    },
    /// Something is at that name and it is not a file SURE reads.
    NotReadableKind {
        /// What is there, as one of this module's own phrases.
        kind: &'static str,
    },
}

impl UnreadReason {
    /// The sentence a person reads.
    ///
    /// Every phrase here is a constant, so no part of a project can write it.
    /// The variable parts — an operating system message, a parser position —
    /// live in the fields and are rendered separately.
    #[must_use]
    pub const fn plain_description(&self) -> &'static str {
        match self {
            Self::OutOfBudget { .. } => {
                "SURE had already read as many project files as it reads in one run, \
                 so it did not read this one."
            }
            Self::TooLarge { .. } => {
                "This file is larger than SURE reads at once, so SURE did not read it."
            }
            Self::Unreadable { .. } => "SURE could not open this file.",
            Self::NotText { .. } => "This file is not text SURE can read.",
            Self::NotParsed { .. } => "This file is not in the format its name promises.",
            Self::WrongShape { .. } => "This file is the right format but the wrong shape.",
            Self::NotReadableKind { .. } => {
                "There is something at this name, and it is not a file SURE reads."
            }
        }
    }

    /// The stable wire name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::OutOfBudget { .. } => "out_of_budget",
            Self::TooLarge { .. } => "too_large",
            Self::Unreadable { .. } => "unreadable",
            Self::NotText { .. } => "not_text",
            Self::NotParsed { .. } => "not_parsed",
            Self::WrongShape { .. } => "wrong_shape",
            Self::NotReadableKind { .. } => "not_readable_kind",
        }
    }

    /// The variable part of the description, when there is one.
    ///
    /// Separate from [`Self::plain_description`] on purpose: this text can come
    /// from the operating system or from a parser, and it is *reported* rather
    /// than *composed into a sentence*. The distinction is the one
    /// `docs/architecture/EVIDENCE_MODEL.md` draws between what SURE says and
    /// what something else said.
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Unreadable { detail } | Self::NotText { detail } | Self::NotParsed { detail } => {
                Some(detail)
            }
            Self::WrongShape { found } => Some(found),
            Self::NotReadableKind { kind } => Some(kind),
            Self::OutOfBudget { .. } | Self::TooLarge { .. } => None,
        }
    }
}

/// What reading one named file produced.
///
/// The four arms are the four answers in the module comment. There is
/// deliberately no `Option<Value>` accessor: a caller that wants the value has
/// to say what it does with the other three.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ReadFile {
    /// Nothing is at that name.
    Absent,
    /// There is a file and this is what it holds.
    Parsed(serde_json::Value),
    /// Something was there and SURE did not get a value out of it.
    Unread(UnreadReason),
}

impl ReadFile {
    /// The value, if there is one.
    pub(super) fn value(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Parsed(value) => Some(value),
            Self::Absent | Self::Unread(_) => None,
        }
    }

    /// The reason, if it was not read.
    pub(super) fn reason(&self) -> Option<&UnreadReason> {
        match self {
            Self::Unread(reason) => Some(reason),
            Self::Absent | Self::Parsed(_) => None,
        }
    }

    /// Whether anything at all was at that name.
    pub(super) fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// What one walk found, arranged for the questions discovery asks of it.
///
/// Built once from a [`Scan`] rather than asking the filesystem again. Two
/// questions are asked of it — *is there a file at this name* and *which
/// directories are directly inside this one* — and both are answered from the
/// walk SURE already took, so discovery cannot disagree with the scan it
/// reports beside it.
///
/// # Names are matched the way the platform matches them
///
/// A project with `Package.json` on Windows is a project with a `package.json`:
/// Windows would open it, and a lookup that missed it would report the project
/// as having no manifest. So the index folds case where the platform does, and
/// the rule comes from [`ScanOptions::case`] rather than from this module, so
/// that both can be tested on one machine.
///
/// [`ScanOptions::case`]: crate::scan::ScanOptions::case
pub(super) struct Tree<'a> {
    /// Every file the walk found, keyed the way the platform compares names.
    files: BTreeMap<String, &'a Path>,
    /// Every directory the walk found and went inside.
    directories: BTreeSet<String>,
    /// The directories directly inside each directory, by the same key.
    children: BTreeMap<String, Vec<&'a Path>>,
    /// The things at a name that are not files SURE reads, and what each is.
    refused: BTreeMap<String, &'static str>,
    case: CaseSensitivity,
}

impl<'a> Tree<'a> {
    /// Arrange a walk for lookup.
    pub(super) fn of(scan: &'a Scan, case: CaseSensitivity) -> Self {
        let mut tree = Self {
            files: BTreeMap::new(),
            directories: BTreeSet::new(),
            children: BTreeMap::new(),
            refused: BTreeMap::new(),
            case,
        };

        for entry in scan.entries() {
            let key = tree.key(&entry.path);
            if entry.kind.is_file() {
                tree.files.insert(key, &entry.path);
            } else {
                tree.directories.insert(key.clone());
                tree.children
                    .entry(tree.key(entry.path.parent().unwrap_or(Path::new(""))))
                    .or_default()
                    .push(&entry.path);
            }
        }
        // The walk's own order is fixed, but the order of a child list is what
        // decides which member of a workspace is read first, and the budget for
        // reading manifests is finite. A child list whose order depended on
        // insertion would make *which* manifests SURE ran out of budget on
        // depend on the scan's internals; sorting by the key the lookup uses
        // makes it a function of the project.
        for list in tree.children.values_mut() {
            list.sort_by_key(|path| lookup_key(path, case));
        }

        for skipped in scan.skipped() {
            let kind = match skipped.reason {
                SkipReason::NotFollowed => "a link",
                SkipReason::SpecialFile => "a pipe, a socket or a device",
                // Every other reason means SURE did not look, which is not the
                // same claim as "there is something here and SURE declined to
                // read it". Recording one of those here would turn "not looked
                // at" into "not a file", and a scan that ran out of budget
                // would report every manifest past the cut as unreadable.
                _ => continue,
            };
            // A skip can be a directory (a link to a directory, say). Those are
            // not in `entries`, so recording them here is the only way the name
            // is answerable at all.
            tree.refused.insert(tree.key(&skipped.path), kind);
        }

        tree
    }

    /// The lookup key for a path, folding case where the platform does.
    fn key(&self, path: &Path) -> String {
        let text = crate::scan::display_path(path);
        match self.case {
            CaseSensitivity::Sensitive => text,
            CaseSensitivity::Insensitive => text.to_lowercase(),
        }
    }

    /// What is at `relative`, without reading it.
    ///
    /// The answer distinguishes *nothing is there* from *there is something
    /// SURE does not read*, which is the whole point of the type. See the module
    /// comment.
    pub(super) fn probe(&self, relative: &Path) -> Probe<'a> {
        let key = self.key(relative);
        if let Some(path) = self.files.get(&key) {
            return Probe::File(path);
        }
        if let Some(kind) = self.refused.get(&key) {
            return Probe::Other(kind);
        }
        if self.directories.contains(&key) {
            return Probe::Other("a directory");
        }
        Probe::Nothing
    }

    /// The directories directly inside `relative`, in a fixed order.
    ///
    /// Empty for a path the walk did not list, which is indistinguishable here
    /// from a path with no subdirectories — and deliberately so: both mean the
    /// same thing to a caller expanding a workspace pattern, which is that this
    /// branch of the pattern names nothing SURE can read.
    pub(super) fn child_directories(&self, relative: &Path) -> &[&'a Path] {
        self.children
            .get(&self.key(relative))
            .map_or(&[], Vec::as_slice)
    }

    /// Whether the walk saw `relative` as a directory.
    pub(super) fn is_directory(&self, relative: &Path) -> bool {
        self.directories.contains(&self.key(relative))
    }

    /// The files directly inside `relative`, in a fixed order.
    ///
    /// For the one question discovery cannot ask by name: *which requirements
    /// files does this project have?* Their names are chosen by the project —
    /// `requirements.txt`, `requirements-dev.txt`, `requirements-prod.txt` —
    /// so there is no fixed list to probe, and an arbitrary list of guesses
    /// would silently miss the one a project actually has.
    ///
    /// Only files: a *directory* named `requirements.txt` is not a requirements
    /// file, and naming it as one would be a claim the walk does not support.
    /// Empty for a path the walk did not list, which is indistinguishable from a
    /// directory with no files in it and deliberately so — see
    /// [`Self::child_directories`] for the same rule.
    pub(super) fn child_files(&self, relative: &Path) -> Vec<&'a Path> {
        let parent = self.key(relative);
        self.files
            .iter()
            .filter(|(_, path)| self.key(path.parent().unwrap_or(Path::new(""))) == parent)
            .map(|(_, path)| *path)
            .collect()
    }
}

/// What is at a name, before reading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Probe<'a> {
    /// Nothing.
    Nothing,
    /// A file, which may be read.
    File(&'a Path),
    /// Something SURE does not read, described by a constant phrase.
    Other(&'static str),
}

/// Read one file as JSON, within the limits.
///
/// `probe` is what [`Tree::probe`] said is at the name — passed in rather than
/// looked up again so that what is read and what was found cannot come from two
/// different walks. It carries the path, which is also why no path is asked for
/// separately: a second one could disagree with it.
pub(super) fn read_json(
    root: &Path,
    probe: Probe<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
) -> ReadFile {
    let text = match read_text(root, probe, options, budget) {
        Ok(text) => text,
        Err(read) => return read,
    };
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => ReadFile::Parsed(value),
        Err(error) => ReadFile::Unread(UnreadReason::NotParsed {
            detail: error.to_string(),
        }),
    }
}

/// Read one file as YAML, within the limits.
///
/// Parsed into the same [`serde_json::Value`] a JSON manifest is, so that the
/// two kinds of manifest are interpreted by one set of accessors rather than by
/// two that agree today. A YAML document whose keys are not strings cannot be
/// represented — `serde_json::Value` requires string keys — and that is reported
/// as [`UnreadReason::NotParsed`] rather than as a document with no keys.
pub(super) fn read_yaml(
    root: &Path,
    probe: Probe<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
) -> ReadFile {
    let text = match read_text(root, probe, options, budget) {
        Ok(text) => text,
        Err(read) => return read,
    };
    match serde_yaml_ng::from_str::<serde_json::Value>(&text) {
        Ok(value) => ReadFile::Parsed(value),
        Err(error) => ReadFile::Unread(UnreadReason::NotParsed {
            detail: error.to_string(),
        }),
    }
}

/// Read one file as TOML, within the limits.
///
/// Parsed into the same [`serde_json::Value`] a JSON or YAML manifest is, so all
/// three kinds of manifest are interpreted by one set of accessors rather than
/// by three that agree today.
///
/// # Why this does not deserialize straight into `serde_json::Value`
///
/// `toml::from_str::<serde_json::Value>` compiles, runs, and **changes values
/// the project declared**, in two ways that were measured rather than assumed
/// before this function was written:
///
/// ```text
/// when = 2024-01-01        -> {"when":{"$__toml_private_datetime":"2024-01-01"}}
/// a = inf                  -> {"a":null}
/// b = nan                  -> {"b":null}
/// ```
///
/// The first invents a table the project did not write, under a key that looks
/// exactly like project data — a manifest with a date in it would be described
/// as declaring that table, and a project that genuinely declared a key of that
/// name would be indistinguishable from one that declared a date. The second
/// turns a value that was there into a value that is not. Both are the failure
/// `CLAUDE.md` ranks above a visible error: not a parse that failed, but a
/// document described as saying something it does not say.
///
/// So the conversion is written out here, and it holds to one rule: **a value
/// that JSON cannot represent is carried as the text TOML wrote it with.** A
/// datetime becomes its own TOML spelling as a string; a non-finite float
/// becomes `inf`, `-inf` or `nan`, which is what the project wrote. Nothing is
/// dropped and nothing is invented. Nothing in a `pyproject.toml` is
/// *interpreted* from it — a version pin whose spelling is a date is still
/// carried verbatim and never resolved.
pub(super) fn read_toml(
    root: &Path,
    probe: Probe<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
) -> ReadFile {
    let text = match read_text(root, probe, options, budget) {
        Ok(text) => text,
        Err(read) => return read,
    };
    match toml::from_str::<toml::Value>(&text) {
        Ok(value) => ReadFile::Parsed(to_json(&value)),
        Err(error) => ReadFile::Unread(UnreadReason::NotParsed {
            detail: error.to_string(),
        }),
    }
}

/// Read one file as text, within the limits.
///
/// The value is a JSON string holding the file's whole text, so that a file
/// whose entire content *is* the claim — `.python-version`, a
/// `requirements.txt` — is read by the same code, within the same limits, and
/// reported by the same [`ReadFile`] arms as every structured manifest. A
/// separate reader for "the file is just text" would be a second place for
/// *absent* and *unreadable* to be confused, which is the whole reason this
/// module exists.
///
/// The text is **not trimmed, split or normalised** here. A caller that wants
/// lines splits them itself, because what a newline means depends on the format
/// it is reading and this function does not know it.
pub(super) fn read_text_file(
    root: &Path,
    probe: Probe<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
) -> ReadFile {
    match read_text(root, probe, options, budget) {
        Ok(text) => ReadFile::Parsed(serde_json::Value::String(text)),
        Err(read) => read,
    }
}

/// A TOML value as the JSON-shaped value everything else here reads.
///
/// Total by construction: every `toml::Value` has an answer, and the arms that
/// have to change the kind of a value say so. See [`read_toml`].
///
/// `pub(super)` rather than private because [`super::python`]'s tests build
/// their fixtures by parsing a TOML string, and going through *this* function
/// means those tests exercise the same conversion the reader uses. A private
/// copy beside them would test a conversion nothing runs.
pub(super) fn to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(text) => serde_json::Value::String(text.clone()),
        toml::Value::Integer(number) => serde_json::Value::Number((*number).into()),
        toml::Value::Float(number) => match serde_json::Number::from_f64(*number) {
            Some(number) => serde_json::Value::Number(number),
            // JSON has no way to write a non-finite number. The string is
            // TOML's own spelling of the same value, so this is the one
            // conversion that loses nothing and invents nothing: `inf` is what
            // the project wrote.
            None => serde_json::Value::String(
                if number.is_nan() {
                    "nan"
                } else if number.is_sign_negative() {
                    "-inf"
                } else {
                    "inf"
                }
                .to_owned(),
            ),
        },
        toml::Value::Boolean(flag) => serde_json::Value::Bool(*flag),
        // A TOML datetime is not a string in TOML and would have to be one in
        // JSON either way. `Display` renders the document's own text rather
        // than a re-formatted instant, so `2024-01-01` stays `2024-01-01`.
        toml::Value::Datetime(when) => serde_json::Value::String(when.to_string()),
        toml::Value::Array(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        toml::Value::Table(entries) => serde_json::Value::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), to_json(value)))
                .collect(),
        ),
    }
}

/// Read one file as text, within the limits.
///
/// The manifest budget is spent here, so a file that is never read is also never
/// counted — a project is not penalised for the manifests it does not have.
///
/// A name with nothing at it, and a name with something SURE does not read at
/// it, are both answered **before** the budget is spent. Neither is a manifest
/// SURE declined to read because it had read too many; charging the budget for
/// them would make a project with many absent optional files exhaust its budget
/// without opening anything.
fn read_text(
    root: &Path,
    probe: Probe<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
) -> Result<String, ReadFile> {
    let entry = match probe {
        Probe::Nothing => return Err(ReadFile::Absent),
        Probe::Other(kind) => {
            return Err(ReadFile::Unread(UnreadReason::NotReadableKind { kind }));
        }
        Probe::File(entry) => entry,
    };

    if !budget.take() {
        return Err(ReadFile::Unread(UnreadReason::OutOfBudget {
            limit: budget.limit,
        }));
    }

    let found = fs::File::open(root.join(entry)).map_err(|error| {
        ReadFile::Unread(UnreadReason::Unreadable {
            detail: error.to_string(),
        })
    })?;

    // One byte past the limit rather than the limit itself, so that a file
    // exactly at the limit is read and a file that grows past it while SURE is
    // reading is still refused. Checking `metadata().len()` first would be a
    // cheaper test that a file could pass and then fail.
    let cap = options.max_manifest_bytes.saturating_add(1);
    let mut bytes = Vec::new();
    found
        .take(cap)
        .read_to_end(&mut bytes)
        .map_err(|error: io::Error| {
            ReadFile::Unread(UnreadReason::Unreadable {
                detail: error.to_string(),
            })
        })?;

    if bytes.len() as u64 > options.max_manifest_bytes {
        return Err(ReadFile::Unread(UnreadReason::TooLarge {
            limit: options.max_manifest_bytes,
        }));
    }

    String::from_utf8(bytes).map_err(|error| {
        ReadFile::Unread(UnreadReason::NotText {
            detail: format!(
                "not valid UTF-8 at byte {}",
                error.utf8_error().valid_up_to()
            ),
        })
    })
}

/// How many manifests one discovery has read.
///
/// A counter rather than a vector of refusals: what was refused is reported by
/// the caller that tried to read it, in the same list as every other unread
/// file, so there is one place a reader looks and one rule for what goes in it.
pub(super) struct Budget {
    limit: usize,
    spent: usize,
}

impl Budget {
    /// A budget for `limit` manifests.
    pub(super) fn new(limit: usize) -> Self {
        Self { limit, spent: 0 }
    }

    /// Spend one manifest, if there is one left to spend.
    fn take(&mut self) -> bool {
        if self.spent >= self.limit {
            return false;
        }
        self.spent += 1;
        true
    }

    /// How many have been spent, for a test to be able to assert the budget was
    /// actually reached.
    #[cfg(test)]
    pub(super) fn spent(&self) -> usize {
        self.spent
    }
}

/// The key a path is looked up by, for a caller that needs to name one.
///
/// Used by the workspace expansion, which compares the path a pattern produced
/// against the paths the walk found. Exposed rather than duplicated so that the
/// folding rule is written once.
pub(super) fn lookup_key(path: &Path, case: CaseSensitivity) -> String {
    let text = crate::scan::display_path(path);
    match case {
        CaseSensitivity::Sensitive => text,
        CaseSensitivity::Insensitive => text.to_lowercase(),
    }
}

/// A path from a manifest, made safe to join onto the project root.
///
/// Returns `None` for anything that is not a plain relative path inside the
/// project: an absolute path, a Windows drive or UNC prefix, or any `..`
/// component. A workspace member list is project-controlled text, and a project
/// that wrote `"workspaces": ["../../elsewhere"]` must not be able to make SURE
/// read a file outside the project — the same containment rule
/// `crate::scan` holds to by construction, applied to a path that arrived as
/// data rather than from the operating system.
pub(super) fn contained_relative(text: &str) -> Option<PathBuf> {
    // `Path::new` on Windows treats `/` as a separator, so a manifest written on
    // any platform is read the same way here.
    let path = Path::new(text);
    if path.is_absolute() || path.has_root() {
        return None;
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => out.push(name),
            // `..`, `.` and any prefix: refused rather than normalised. A
            // normalised `..` is still a climb, and a `CurDir` would make
            // `a/./b` and `a/b` two spellings of one path in a report.
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Parse a TOML document the way [`read_toml`] does, without a file.
    fn toml_value(text: &str) -> serde_json::Value {
        to_json(&toml::from_str::<toml::Value>(text).expect("the test document must parse"))
    }

    #[test]
    fn a_toml_datetime_is_carried_as_the_text_that_was_written() {
        // The claim this holds, and the reason `read_toml` does not deserialize
        // straight into `serde_json::Value`: that path turns every date into
        // `{"$__toml_private_datetime": "..."}`, a table the project did not
        // write. A manifest with a release date in it would be described as
        // declaring a table, and a project that genuinely declared a key of
        // that name would read identically to one that declared a date.
        for (written, text) in [
            ("2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"),
            ("2024-01-01", "2024-01-01"),
            ("07:32:00", "07:32:00"),
            (
                "1979-05-27T07:32:00.999999-07:00",
                "1979-05-27T07:32:00.999999-07:00",
            ),
        ] {
            let document = format!("when = {written}\n");
            assert_eq!(
                toml_value(&document),
                serde_json::json!({ "when": text }),
                "{written} did not survive the read as text"
            );
        }
        // The premise, asserted rather than described: the direct conversion
        // really does invent that table. If a future `toml` changes this, the
        // comment above is what has to be rewritten, not the behaviour.
        let invented = toml::from_str::<serde_json::Value>("when = 2024-01-01\n")
            .expect("the document parses");
        assert_eq!(
            invented,
            serde_json::json!({ "when": { "$__toml_private_datetime": "2024-01-01" } }),
            "the reason this function exists no longer holds; re-measure before \
             simplifying it away"
        );
    }

    #[test]
    fn a_toml_number_json_cannot_write_is_carried_as_the_text_that_was_written() {
        // `inf` and `nan` become `null` under the direct conversion, which is a
        // value that was declared becoming a value that was not.
        assert_eq!(
            toml_value("a = inf\nb = -inf\nc = nan\n"),
            serde_json::json!({ "a": "inf", "b": "-inf", "c": "nan" })
        );
        // And the ordinary numbers are still numbers, so the rule above is not
        // costing every float its type.
        assert_eq!(
            toml_value("a = 1.5\nb = 3\nc = -0.25\nd = 1e10\n"),
            serde_json::json!({ "a": 1.5, "b": 3, "c": -0.25, "d": 1e10_f64 })
        );
        assert_eq!(
            toml::from_str::<serde_json::Value>("a = inf\n").expect("the document parses"),
            serde_json::json!({ "a": null }),
            "the reason this function exists no longer holds; re-measure before \
             simplifying it away"
        );
    }

    #[test]
    fn a_toml_document_keeps_its_shape_its_order_and_its_types() {
        // The rest of the conversion is meant to be unremarkable, which is why
        // it is asserted: a conversion that quietly flattened a nested table
        // would be found by this and not by the two tests above.
        let value = toml_value(
            "[project]\n\
             name = \"x\"\n\
             dependencies = [\"a>=1\", \"b\"]\n\
             \n\
             [project.optional-dependencies]\n\
             dev = [\"pytest\"]\n\
             \n\
             [tool.poetry]\n\
             name = \"x\"\n\
             \n\
             [[tool.uv.workspace.members]]\n",
        );
        assert_eq!(
            value,
            serde_json::json!({
                "project": {
                    "name": "x",
                    "dependencies": ["a>=1", "b"],
                    "optional-dependencies": { "dev": ["pytest"] }
                },
                "tool": { "poetry": { "name": "x" }, "uv": { "workspace": { "members": [{}] } } }
            })
        );
        assert_eq!(
            value.pointer("/project/dependencies/0"),
            Some(&serde_json::json!("a>=1")),
            "a version requirement is carried verbatim, never resolved"
        );
    }

    #[test]
    fn a_toml_document_that_is_not_valid_toml_is_unread_and_never_absent() {
        // The same rule the JSON and YAML readers hold to. A `pyproject.toml`
        // with a syntax error in it is a file that is there and was not read.
        let broken = toml::from_str::<toml::Value>("[project\nname = \"x\"\n");
        assert!(broken.is_err(), "the fixture must not parse");
    }

    #[test]
    fn a_path_a_manifest_named_cannot_leave_the_project() {
        assert_eq!(
            contained_relative("packages/app"),
            Some(PathBuf::from("packages").join("app"))
        );
        // A separator from either platform, because a manifest is written by a
        // person on whichever one they use.
        assert!(contained_relative("packages/app").is_some());
        assert!(contained_relative(r"packages\app").is_some());

        // Climbing, spelled the way every platform spells it.
        for climbing in [
            "../elsewhere",
            "packages/../../elsewhere",
            "..",
            "./..",
            "/etc/passwd",
            "",
            ".",
            "./",
        ] {
            assert!(
                contained_relative(climbing).is_none(),
                "{climbing:?} was accepted as a path inside the project"
            );
        }

        // **A Windows-spelled absolute path is not absolute on Unix.** There it
        // is one relative name with a backslash in it, which is a legal name for
        // something inside the project, and reading it as one is correct as well
        // as harmless: the caller looks for a directory by that name and finds
        // nothing.
        //
        // The first version of this test asserted the Windows answer on every
        // platform, and **it failed on the macOS and Ubuntu CI jobs** with
        // `"C:\\Windows" was accepted as a path inside the project`. The code was
        // right and the assertion was wrong -- which is the direction to be wrong
        // in, and still a red CI run that a Windows-only check could not have
        // predicted. It is now pinned both ways, because the tempting
        // "portability fix" here is to refuse these everywhere, and that would
        // refuse a legal name on the platform the manifest was written for.
        #[cfg(windows)]
        for windows_absolute in [r"C:\Windows", r"\\server\share"] {
            assert!(
                contained_relative(windows_absolute).is_none(),
                "{windows_absolute:?} is absolute on Windows and cannot be inside \
                 the project"
            );
        }
        #[cfg(unix)]
        for windows_spelled in [r"C:\Windows", r"\\server\share"] {
            // The premise, asserted rather than assumed: on this platform the
            // name is not absolute, which is *why* accepting it is right.
            assert!(
                !Path::new(windows_spelled).is_absolute(),
                "premise: {windows_spelled:?} is relative on this platform"
            );
            // And the contract, rather than the exact spelling the first draft
            // of this test guessed at: whatever comes back is a path that cannot
            // leave the project. That is the whole of what the function
            // promises, and it is the statement that has to hold on a platform
            // this machine cannot run.
            let inside = contained_relative(windows_spelled)
                .expect("a relative name is a name the project could hold");
            assert!(
                !inside.is_absolute() && !inside.has_root(),
                "{windows_spelled:?} produced {inside:?}, which is not contained"
            );
        }
    }

    #[test]
    fn the_lookup_key_folds_case_exactly_where_the_platform_does() {
        let upper = Path::new("Package.JSON");
        let lower = Path::new("package.json");
        assert_ne!(
            lookup_key(upper, CaseSensitivity::Sensitive),
            lookup_key(lower, CaseSensitivity::Sensitive),
            "on a case-sensitive filesystem these are two files"
        );
        assert_eq!(
            lookup_key(upper, CaseSensitivity::Insensitive),
            lookup_key(lower, CaseSensitivity::Insensitive),
            "on a case-insensitive filesystem these are one file, and missing it \
             would report a project as having no manifest"
        );
    }

    #[test]
    fn the_budget_stops_counting_rather_than_wrapping() {
        let mut budget = Budget::new(2);
        assert!(budget.take());
        assert!(budget.take());
        assert!(!budget.take(), "the third read was allowed");
        assert!(!budget.take(), "the fourth read was allowed");
        assert_eq!(budget.spent(), 2, "a refused read was counted as spent");
    }

    #[test]
    fn a_budget_of_zero_reads_nothing_and_says_so_rather_than_reading_anything() {
        let mut budget = Budget::new(0);
        assert!(!budget.take());
        assert_eq!(budget.spent(), 0);
    }

    #[test]
    fn every_reason_says_which_of_the_four_answers_it_is() {
        // The rule this module exists for, asserted over the whole enum: an
        // unread file is never reported as an absent one, and the two are
        // distinguishable by a caller.
        let reasons = [
            UnreadReason::OutOfBudget { limit: 1 },
            UnreadReason::TooLarge { limit: 1 },
            UnreadReason::Unreadable {
                detail: "no".to_owned(),
            },
            UnreadReason::NotText {
                detail: "no".to_owned(),
            },
            UnreadReason::NotParsed {
                detail: "no".to_owned(),
            },
            UnreadReason::WrongShape { found: "an array" },
            UnreadReason::NotReadableKind { kind: "a link" },
        ];
        for reason in reasons {
            assert!(!reason.as_str().is_empty());
            assert!(!reason.plain_description().is_empty());
            let read = ReadFile::Unread(reason.clone());
            assert!(!read.is_absent(), "{reason:?} was reported as absent");
            assert_eq!(read.reason(), Some(&reason));
            assert!(read.value().is_none());
        }
        assert!(ReadFile::Absent.is_absent());
        assert!(ReadFile::Absent.reason().is_none());
        assert!(
            !ReadFile::Parsed(serde_json::json!({})).is_absent(),
            "a file that was read was reported as absent"
        );
    }
}

//! The keys a project reads out of the environment, and where it says they exist.
//!
//! Step 1 of `docs/architecture/CHECK_PIPELINE.md` lists *"config references"*
//! among the things discovery produces, and `docs/product/MVP_SPEC.md` lists
//! *"environment-variable/config references"* among the checks. Two documents in
//! `docs/security/` set the terms both of those are held to:
//! `SECRET_REDACTION.md` — *"avoid logging raw environment values"*, *"never
//! claim perfect secret detection"* — and `PRIVACY.md`, which keeps source and
//! evidence on this machine.
//!
//! # The one rule everything here is arranged around
//!
//! **A key is recorded. A value is not held at all.**
//!
//! That is not the same as *"values are not reported"*, which would be a promise
//! about the code that writes a report and would be broken by the first
//! `format!("{reference:?}")` somebody added for a log line. There is no field
//! on [`Reference`] or [`Declaration`] that can hold a value, so there is
//! nothing for a report to leak and nothing for a future edit to start printing.
//! The extractors go further and discard the rest of the line before they
//! return: [`key_from_env_line`] takes the text before the first `=` and hands
//! back only that, so a line reading `API_KEY=sk-live-…` produces the seven
//! characters of `API_KEY` and the value is never a `String` anywhere in this
//! module.
//!
//! The second half of the rule is what SURE **opens on disk**. A `.env` file is
//! where a project's real values live. This module reads a `.env`-family file
//! only when its name marks it a *template* — [`is_env_template`] — and `.env`,
//! `.env.local`, `.env.production` and `.env.test` are all outside that rule.
//! They are not read, not parsed, and not reported as *unread* either, because
//! declining to open the file where the secrets are is a decision rather than a
//! loss of coverage.
//!
//! # What the comparison is, and what it is not
//!
//! [`ReferenceReport::of`] reads two sets of files and compares them:
//!
//! - **the reading side** — source files (`.js`, `.ts`, `.py`, `.rs` and the
//!   rest of [`ReadForm`]'s families), where a key shows up as
//!   `process.env.NAME`, `os.environ["NAME"]`, `env::var("NAME")` and the other
//!   forms [`ReadForm`] lists.
//! - **the declaration side** — `.env`-style templates and Markdown, where a
//!   project says a key exists.
//!
//! A key found on one side only becomes [`KeyStatus::ReadButNotDeclared`] or
//! [`KeyStatus::DeclaredButNotRead`]. **Neither is a claim about the project
//! unless the reading was complete.** A file that was too large, a file whose
//! bytes were not UTF-8, a run that spent its budget, a directory the scan could
//! not walk — each of those means *SURE did not look*, and a report that renders
//! the absence as a finding is making a claim about a project it did not finish
//! reading. [`ReferenceReport::is_complete`] is the question a caller has to ask
//! first, and [`ReferenceReport::unread`] is where the answer is.
//!
//! # What it does not do
//!
//! **It does not parse anything.** A needle is matched textually, so a key
//! mention inside a comment, inside a string, or inside a documentation example
//! that this module read as a source file is counted as a read. There is a test
//! that pins that behaviour (`a_mention_inside_a_comment_is_still_a_read`) so
//! that a reader meets it as a stated limit rather than as a surprise.
//!
//! **It does not know what a key means.** `DATABASE_URL` and `PORT` are two
//! strings. SURE has read no schema, no compose file and no deployment
//! manifest, so it cannot say that a declared key is spelled the way some
//! consumer spells it.
//!
//! **It does not read the environment.** `std::env::var` appears nowhere in this
//! module: what SURE would find there is this machine's values, and the question
//! is what the *project* asks for.
//!
//! **It does not read `.env`.** See above — that is the point rather than an
//! omission.
//!
//! **It does not decide whether a key is a secret.** `redact::looks_like_credential_name`
//! exists for names SURE is about to put in one of *its own* messages. A key
//! such as `AWS_SECRET_ACCESS_KEY` is a finding about the project and is
//! recorded as the name it is; suppressing it would hide the thing worth
//! knowing. The sentences in this module are constants, so no key name reaches
//! a message that `redact` would have to clean.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::discover::{Discovery, Ecosystem};
use crate::scan::{Scan, display_path};

/// One place a source file asks the environment for a key.
///
/// There is no `value` field, no `default` field and no `line_text` field. Each
/// of the three is a way a value reaches a report; see the module docs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reference {
    /// The key, exactly as the source file spelled it.
    pub name: String,
    /// Which call the key was found in.
    pub form: ReadForm,
    /// The file, relative to the project root.
    pub path: PathBuf,
    /// The 1-based line the read is on.
    pub line: usize,
}

impl Reference {
    /// The file as text, with `/` on every platform.
    #[must_use]
    pub fn display_path(&self) -> String {
        display_path(&self.path)
    }
}

/// The calls this module recognises, grouped by the family of call it is.
///
/// The grouping is by *family* and not by literal spelling: [`Self::RustEnv`]
/// covers `env::var`, `std::env::var`, `env::var_os` and `std::env::var_os`,
/// because the four differ in what they return rather than in which key they ask
/// for, and a report that split them would be reporting a distinction the
/// finding does not have. Every family's spellings are listed here so that what
/// SURE looked for is a readable list rather than a regular expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadForm {
    /// `process.env.NAME` — a property read on the Node environment object.
    NodeProperty,
    /// `process.env["NAME"]` and `process.env['NAME']` — a computed read on it.
    NodeComputed,
    /// `os.environ["NAME"]` and `os.environ.get("NAME")` — Python.
    PythonEnviron,
    /// `os.getenv("NAME")` — Python.
    PythonGetenv,
    /// `env::var("NAME")`, `std::env::var("NAME")`, `env::var_os("NAME")` and
    /// `std::env::var_os("NAME")` — Rust.
    RustEnv,
}

impl ReadForm {
    /// Every form this build recognises, in a fixed order.
    pub const ALL: &'static [Self] = &[
        Self::NodeProperty,
        Self::NodeComputed,
        Self::PythonEnviron,
        Self::PythonGetenv,
        Self::RustEnv,
    ];

    /// A short stable name for this form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NodeProperty => "process.env",
            Self::NodeComputed => "process.env[...]",
            Self::PythonEnviron => "os.environ",
            Self::PythonGetenv => "os.getenv",
            Self::RustEnv => "env::var",
        }
    }

    /// The ecosystem whose source files this form is looked for in.
    ///
    /// A derivation rather than a second stored field: a form that named one
    /// ecosystem in its variant and another here would be a bug nobody could
    /// see.
    #[must_use]
    pub const fn ecosystem(self) -> Ecosystem {
        match self {
            Self::NodeProperty | Self::NodeComputed => Ecosystem::Node,
            Self::PythonEnviron | Self::PythonGetenv => Ecosystem::Python,
            Self::RustEnv => Ecosystem::Rust,
        }
    }

    /// What this form is, as a fragment of a sentence.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::NodeProperty => {
                "a property read on the Node environment object, as in process.env.NAME"
            }
            Self::NodeComputed => {
                "a computed read on the Node environment object, as in process.env[\"NAME\"]"
            }
            Self::PythonEnviron => {
                "a read of Python's environment mapping, as in os.environ[\"NAME\"] \
                 or os.environ.get(\"NAME\")"
            }
            Self::PythonGetenv => "a call to os.getenv, as in os.getenv(\"NAME\")",
            Self::RustEnv => "a call to env::var or env::var_os, as in env::var(\"NAME\")",
        }
    }
}

/// One place the project says a key exists.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Declaration {
    /// The key, exactly as the declaring file spelled it.
    pub name: String,
    /// Which kind of file declared it.
    pub kind: DeclarationKind,
    /// The file, relative to the project root.
    pub path: PathBuf,
    /// The 1-based line the declaration is on.
    pub line: usize,
}

impl Declaration {
    /// The file as text, with `/` on every platform.
    #[must_use]
    pub fn display_path(&self) -> String {
        display_path(&self.path)
    }
}

/// Where a declaration was found.
///
/// Neither kind is authoritative. A template is a file somebody has to keep
/// current and a document is prose, and both can be wrong about a project in
/// ways SURE cannot see: this module compares the two sides of a project's own
/// writing against each other, not against a specification it does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationKind {
    /// A `.env`-style template: `.env.example`, `.env.sample` and the rest of
    /// the names [`is_env_template`] accepts.
    Example,
    /// A document SURE read: a `README*` file, or a Markdown file.
    Document,
}

impl DeclarationKind {
    /// A short stable name for this kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Example => "example",
            Self::Document => "document",
        }
    }

    /// What this kind of file is, as a fragment of a sentence.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Example => "an example environment file",
            Self::Document => "a document",
        }
    }
}

/// What the two sides say about one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyStatus {
    /// A source file reads it and a declaring file names it.
    ReadAndDeclared,
    /// A source file reads it, and no declaring file SURE read names it.
    ///
    /// **This is the arm that must not be rendered as "missing".**
    /// [`ReferenceReport::is_complete`] says whether *no declaring file* is a
    /// statement about the project or only about what SURE managed to read, and
    /// a report that drops that question is making the first claim with the
    /// second one's evidence.
    ReadButNotDeclared,
    /// A declaring file names it, and no source file SURE read reads it.
    ///
    /// Carries the same caveat as [`Self::ReadButNotDeclared`], on the other
    /// side: whether *no source file* is a fact about the project depends on
    /// whether every source file was read.
    DeclaredButNotRead,
}

impl KeyStatus {
    /// Every status, in a fixed order.
    pub const ALL: &'static [Self] = &[
        Self::ReadAndDeclared,
        Self::ReadButNotDeclared,
        Self::DeclaredButNotRead,
    ];

    /// A short stable name for this status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadAndDeclared => "read_and_declared",
            Self::ReadButNotDeclared => "read_but_not_declared",
            Self::DeclaredButNotRead => "declared_but_not_read",
        }
    }

    /// What this status says, as a sentence.
    ///
    /// The two one-sided arms are worded so that they describe **the two lists**
    /// rather than the project, which is the strongest thing that is true
    /// without asking [`ReferenceReport::is_complete`].
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::ReadAndDeclared => {
                "A source file asks for this key and a file in the project names it."
            }
            Self::ReadButNotDeclared => {
                "A source file asks for this key, and no file in the project that SURE \
                 read names it."
            }
            Self::DeclaredButNotRead => {
                "A file in the project names this key, and no source file that SURE read \
                 asks for it."
            }
        }
    }
}

/// One key, everything said about it, and what the two sides make of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyReport {
    /// The key.
    pub name: String,
    /// Every place a source file reads it, in file-and-line order. Empty when
    /// nothing read it.
    pub reads: Vec<Reference>,
    /// Every place a declaring file names it, in file-and-line order. Empty when
    /// nothing declared it.
    pub declarations: Vec<Declaration>,
    /// What the two sides make of it.
    pub status: KeyStatus,
}

impl KeyReport {
    /// Whether a source file SURE read asks for this key.
    ///
    /// Not the same question as [`Self::status`], which also weighs the other
    /// side. This answer is sound whatever the reading's completeness, because
    /// finding a read is a positive finding.
    #[must_use]
    pub fn is_read(&self) -> bool {
        !self.reads.is_empty()
    }

    /// Whether a declaring file SURE read names this key.
    ///
    /// Sound for the same reason as [`Self::is_read`].
    #[must_use]
    pub fn is_declared(&self) -> bool {
        !self.declarations.is_empty()
    }

    /// The first place a source file reads it, for a report that names one.
    #[must_use]
    pub fn first_read(&self) -> Option<&Reference> {
        self.reads.first()
    }
}

/// A file this pass meant to read and did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unread {
    /// The file, relative to the project root.
    pub path: PathBuf,
    /// Why it was not read.
    pub reason: UnreadReason,
}

impl Unread {
    /// The file as text, with `/` on every platform.
    #[must_use]
    pub fn display_path(&self) -> String {
        display_path(&self.path)
    }
}

/// Why a file this pass meant to read was not read.
///
/// Every arm here loses coverage, so every arm makes
/// [`ReferenceReport::is_complete`] false. There is no arm for a file this pass
/// declined to open on purpose — the `.env` family is not *unread*, it is not a
/// candidate, and listing it here would report a decision as a loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnreadReason {
    /// The pass had already read as many files as it will in one run.
    ///
    /// The count is of files, not bytes. A project can hold an unbounded number
    /// of source files and documents, and a pass that read all of them would
    /// scale with the project rather than with the question.
    OutOfBudget {
        /// How many files one pass reads.
        limit: usize,
    },
    /// The pass had already read as many bytes as it will in one run.
    ///
    /// The limit is on the whole pass rather than on one file, because a
    /// thousand files just under [`ReferenceOptions::max_file_bytes`] is a
    /// thousand megabyte reads and the per-file limit does not see it.
    OutOfBytes {
        /// How many bytes one pass reads.
        limit: u64,
    },
    /// Bigger than [`ReferenceOptions::max_file_bytes`].
    ///
    /// The size is checked against the file's metadata before it is opened, so
    /// the number here is the limit that was reached rather than a measurement
    /// of the file: a file one byte over and a file a gigabyte over are refused
    /// at the same point and SURE has read neither.
    TooLarge {
        /// The limit that was reached.
        limit: u64,
    },
    /// The operating system would not open it.
    Unreadable {
        /// The operating system's own words, carried as data.
        detail: String,
    },
    /// The bytes are not UTF-8 text.
    ///
    /// Every language this module reads has a UTF-8 source convention, so this
    /// says something about the file rather than about SURE's encoding support.
    /// Decoding lossily would be worse than reporting the file here: a
    /// replacement character could land inside a key and change which key a
    /// report names, silently.
    NotText {
        /// What the decoder said, carried as data.
        detail: String,
    },
}

impl UnreadReason {
    /// What this reason is, as a sentence.
    #[must_use]
    pub const fn plain_description(&self) -> &'static str {
        match self {
            Self::OutOfBudget { .. } => {
                "SURE had already read as many files as it reads in one pass"
            }
            Self::OutOfBytes { .. } => {
                "SURE had already read as many bytes as it reads in one pass"
            }
            Self::TooLarge { .. } => "the file is bigger than SURE reads in one file",
            Self::Unreadable { .. } => "the operating system would not open it",
            Self::NotText { .. } => "the bytes are not UTF-8 text",
        }
    }
}

/// Limits on one pass.
///
/// The defaults are sized so that a project of ordinary shape is read whole and
/// a project built to be expensive is bounded. They are *not* sized to the
/// machine: a pass that consumed whatever memory happened to be free would be a
/// pass whose cost a user could not predict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceOptions {
    /// The largest file this pass will open. Default 1 MiB.
    ///
    /// A source file larger than this is a generated file or a bundle, and the
    /// keys in it are the keys in whatever generated it.
    pub max_file_bytes: u64,
    /// How many files one pass will read. Default 512.
    pub max_files: usize,
    /// How many bytes one pass will read in total. Default 16 MiB.
    pub max_total_bytes: u64,
}

impl Default for ReferenceOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 1024 * 1024,
            max_files: 512,
            max_total_bytes: 16 * 1024 * 1024,
        }
    }
}

impl ReferenceOptions {
    /// The same options with a different per-file size limit.
    #[must_use]
    pub const fn with_max_file_bytes(mut self, max_file_bytes: u64) -> Self {
        self.max_file_bytes = max_file_bytes;
        self
    }

    /// The same options with a different file count.
    #[must_use]
    pub const fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /// The same options with a different total byte budget.
    #[must_use]
    pub const fn with_max_total_bytes(mut self, max_total_bytes: u64) -> Self {
        self.max_total_bytes = max_total_bytes;
        self
    }
}

/// Every key a project reads, every key it declares, and what the two sides say.
///
/// Built by [`Self::of`], which is the only door: the report is a result rather
/// than a value a caller assembles, so there is no way to produce one whose
/// [`Self::is_complete`] does not match the files it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceReport {
    /// The project root the paths are relative to.
    pub root: PathBuf,
    /// Every key either side named, sorted by key.
    pub keys: Vec<KeyReport>,
    /// Every file this pass meant to read and did not.
    pub unread: Vec<Unread>,
    /// The limits this pass ran under.
    pub options: ReferenceOptions,
    /// Whether the scan underneath this pass walked everything it set out to.
    ///
    /// Private because it is only meaningful through [`Self::is_complete`]: a
    /// directory the walk could not enter holds source files, and a caller that
    /// read this field and nothing else would be asking half the question.
    scan_was_complete: bool,
}

impl ReferenceReport {
    /// Read a discovery's files and compare the two sides, under the defaults.
    #[must_use]
    pub fn of(discovery: &Discovery) -> Self {
        Self::with_options(discovery, &ReferenceOptions::default())
    }

    /// Read a discovery's files and compare the two sides.
    ///
    /// The scan in `discovery` decides *which* files are candidates; this pass
    /// opens them. Nothing is executed, nothing is resolved, and the only files
    /// opened are the ones [`is_source_candidate`] and [`declaration_candidate`]
    /// accept.
    #[must_use]
    pub fn with_options(discovery: &Discovery, options: &ReferenceOptions) -> Self {
        let mut sides: BTreeMap<String, Sides> = BTreeMap::new();
        let mut unread = Vec::new();

        let mut reader = Reader::new(options);
        for candidate in candidates(&discovery.scan) {
            let text = match reader.read(discovery, &candidate) {
                Ok(text) => text,
                Err(reason) => {
                    unread.push(Unread {
                        path: candidate.path,
                        reason,
                    });
                    continue;
                }
            };
            match candidate.side {
                Side::Reading => {
                    for reference in references_in(&text, &candidate.path) {
                        sides
                            .entry(reference.name.clone())
                            .or_default()
                            .reads
                            .push(reference);
                    }
                }
                Side::Declaring(kind) => {
                    for declaration in declarations_in(&text, &candidate.path, kind) {
                        sides
                            .entry(declaration.name.clone())
                            .or_default()
                            .declarations
                            .push(declaration);
                    }
                }
            }
        }

        let keys = sides
            .into_iter()
            .map(|(name, mut sides)| {
                // By file and line rather than by the derived order, which
                // would put every property read in the project before every
                // computed one. A person reading a key's reads wants to walk
                // the files.
                sides
                    .reads
                    .sort_by(|a, b| (&a.path, a.line, a.form).cmp(&(&b.path, b.line, b.form)));
                sides
                    .declarations
                    .sort_by(|a, b| (&a.path, a.line, a.kind).cmp(&(&b.path, b.line, b.kind)));
                let status = match (sides.reads.is_empty(), sides.declarations.is_empty()) {
                    (false, false) => KeyStatus::ReadAndDeclared,
                    (false, true) => KeyStatus::ReadButNotDeclared,
                    (true, false) => KeyStatus::DeclaredButNotRead,
                    // Unreachable by construction: a key is in the map because
                    // one side put it there, and `every_key_came_from_one_of_two_sides`
                    // is what holds that. Written as an arm rather than as a
                    // panic, because saying the smaller wrong thing beats
                    // stopping a check.
                    (true, true) => KeyStatus::DeclaredButNotRead,
                };
                KeyReport {
                    name,
                    reads: sides.reads,
                    declarations: sides.declarations,
                    status,
                }
            })
            .collect();

        Self {
            root: discovery.root.clone(),
            keys,
            unread,
            options: *options,
            scan_was_complete: discovery.scan.is_complete(),
        }
    }

    /// The key with this name, if either side named one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&KeyReport> {
        self.keys
            .binary_search_by(|key| key.name.as_str().cmp(name))
            .ok()
            .map(|index| &self.keys[index])
    }

    /// Every key, sorted by name.
    #[must_use]
    pub fn keys(&self) -> &[KeyReport] {
        &self.keys
    }

    /// Every key with this status, in name order.
    pub fn with_status(&self, status: KeyStatus) -> impl Iterator<Item = &KeyReport> {
        self.keys.iter().filter(move |key| key.status == status)
    }

    /// Whether everything this pass set out to read was read.
    ///
    /// **A caller that reports an absence without asking this first is making a
    /// claim about a project SURE did not finish reading.** False when any
    /// candidate file went unread for any reason, and false when the scan
    /// underneath could not walk some part of the project.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unread.is_empty() && self.scan_was_complete
    }

    /// Every file this pass meant to read and did not.
    #[must_use]
    pub fn unread(&self) -> &[Unread] {
        &self.unread
    }

    /// What the two sides found, in one sentence.
    ///
    /// The counts are data and the sentence is a constant, and the sentence is
    /// worded so that it is true of a partial reading: it says how many keys
    /// *SURE read*, never how many the project has.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let read = self.keys.iter().filter(|key| key.is_read()).count();
        let declared = self.keys.iter().filter(|key| key.is_declared()).count();
        let mut sentence = format!(
            "SURE found {read} environment or configuration {} asked for in source files \
             and {declared} named in the project's examples or documents",
            if read == 1 { "key" } else { "keys" }
        );
        if !self.is_complete() {
            sentence.push_str(
                ", and did not finish reading the project, so a key it did not find may be \
                 one it did not look for",
            );
        }
        sentence.push('.');
        sentence
    }
}

/// One key's two sides, while the map is being built.
#[derive(Debug, Default)]
struct Sides {
    reads: Vec<Reference>,
    declarations: Vec<Declaration>,
}

/// Which side of the comparison a candidate file is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    /// A source file, searched for reads.
    Reading,
    /// A declaring file, searched for names.
    Declaring(DeclarationKind),
}

/// A file this pass will open, with the side it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Candidate {
    path: PathBuf,
    side: Side,
}

/// Every candidate in a scan, in a fixed order.
///
/// Sorted by path so that two runs over the same project read the files in the
/// same order. The budget makes that matter: which file is the one that runs out
/// of budget is a result, and it must not depend on the order a directory
/// happened to come back in.
fn candidates(scan: &Scan) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = scan
        .files()
        .filter_map(|entry| {
            let side = if is_source_candidate(&entry.path) {
                Side::Reading
            } else {
                Side::Declaring(declaration_candidate(&entry.path)?)
            };
            Some(Candidate {
                path: entry.path.clone(),
                side,
            })
        })
        .collect();
    found.sort();
    found
}

/// Whether a file is a source file whose calls this module looks for.
///
/// By extension, because that is what decides which of [`ReadForm`]'s families
/// could appear in it. A `.md` file quoting `process.env.FOO` is not a source
/// file and is read as a document instead — the two sides are told apart by
/// this predicate, so one file cannot be evidence for both.
#[must_use]
pub fn is_source_candidate(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };
    let extension = extension.to_string_lossy().to_lowercase();
    NODE_EXTENSIONS.contains(&extension.as_str())
        || PYTHON_EXTENSIONS.contains(&extension.as_str())
        || RUST_EXTENSIONS.contains(&extension.as_str())
}

/// JavaScript and TypeScript, including the module and declaration variants.
const NODE_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"];

/// Python, including stub files.
const PYTHON_EXTENSIONS: &[&str] = &["py", "pyi"];

/// Rust.
const RUST_EXTENSIONS: &[&str] = &["rs"];

/// Which kind of declaring file this is, or `None` if it declares nothing.
#[must_use]
pub fn declaration_candidate(path: &Path) -> Option<DeclarationKind> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    if is_env_template(&name) {
        return Some(DeclarationKind::Example);
    }
    if name.starts_with("readme") || name.ends_with(".md") {
        return Some(DeclarationKind::Document);
    }
    None
}

/// Whether a file's name marks it a `.env` *template* rather than a real one.
///
/// The rule is that the name is `.env` followed by a dot and at least one
/// further part, one of which is a marker word: `.env.example`, `.env.sample`,
/// `.env.template`, `.env.dist`, `.env.defaults`, and compound names such as
/// `.env.local.example`.
///
/// **What it deliberately excludes is the file the values are in.** `.env` has
/// no part after it, `.env.local` and `.env.production` have parts that are not
/// markers, and `.env.test` is excluded even though it is often a template —
/// because it is also often where a project's real test credentials live, and
/// the two cases are not distinguishable from the name. Every one of those is
/// left unopened, and none of them is reported as *unread*: declining to open
/// the file where the secrets are is a decision, not a loss of coverage.
#[must_use]
pub fn is_env_template(name: &str) -> bool {
    let name = name.to_lowercase();
    let Some(rest) = name.strip_prefix(".env.") else {
        return false;
    };
    rest.split('.')
        .any(|part| ENV_TEMPLATE_MARKERS.contains(&part))
}

/// The words that mark a `.env`-family name as a template.
const ENV_TEMPLATE_MARKERS: &[&str] = &["example", "sample", "template", "dist", "defaults"];

/// Reads candidate files, spending a budget and saying what it did not read.
struct Reader<'a> {
    options: &'a ReferenceOptions,
    files_read: usize,
    bytes_read: u64,
}

impl Reader<'_> {
    fn new(options: &ReferenceOptions) -> Reader<'_> {
        Reader {
            options,
            files_read: 0,
            bytes_read: 0,
        }
    }

    fn read(
        &mut self,
        discovery: &Discovery,
        candidate: &Candidate,
    ) -> Result<String, UnreadReason> {
        if self.files_read >= self.options.max_files {
            return Err(UnreadReason::OutOfBudget {
                limit: self.options.max_files,
            });
        }
        if self.bytes_read >= self.options.max_total_bytes {
            return Err(UnreadReason::OutOfBytes {
                limit: self.options.max_total_bytes,
            });
        }

        let full = discovery.root.join(&candidate.path);
        let limit = self.options.max_file_bytes;
        match fs::metadata(&full) {
            Ok(metadata) if metadata.len() > limit => {
                return Err(UnreadReason::TooLarge { limit });
            }
            Ok(_) => {}
            Err(error) => {
                return Err(UnreadReason::Unreadable {
                    detail: error.to_string(),
                });
            }
        }

        let bytes = fs::read(&full).map_err(|error| UnreadReason::Unreadable {
            detail: error.to_string(),
        })?;
        let text = String::from_utf8(bytes).map_err(|error| UnreadReason::NotText {
            detail: error.utf8_error().to_string(),
        })?;

        self.files_read += 1;
        self.bytes_read += u64::try_from(text.len()).unwrap_or(u64::MAX);
        Ok(text)
    }
}

/// Every read in one source file, in line order.
///
/// Line by line rather than over the whole text: the line number is then known
/// without searching backwards from each match, and a call split across a line
/// break is not a call anybody wrote. CRLF needs no handling because
/// [`str::lines`] strips the carriage return, which matters on the platform this
/// is developed on.
fn references_in(text: &str, path: &Path) -> Vec<Reference> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some((_, form, name, end)) = next_read(rest) {
            found.push(Reference {
                name,
                form,
                path: path.to_path_buf(),
                line: index + 1,
            });
            rest = &rest[end..];
        }
    }
    found
}

/// Every declaration in one declaring file, in line order.
fn declarations_in(text: &str, path: &Path, kind: DeclarationKind) -> Vec<Declaration> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let names = match kind {
            DeclarationKind::Example => key_from_env_line(line).into_iter().collect(),
            DeclarationKind::Document => keys_from_setting_line(line),
        };
        for name in names {
            found.push(Declaration {
                name,
                kind,
                path: path.to_path_buf(),
                line: index + 1,
            });
        }
    }
    found
}

/// The first read in `text`, the form it is, the key, and where the key ends.
///
/// `None` when there is no read left in `text`. The offsets are into `text`, so
/// a caller walking a line can resume at the returned end and find the next one.
fn next_read(text: &str) -> Option<(usize, ReadForm, String, usize)> {
    let mut best: Option<(usize, ReadForm, String, usize)> = None;
    for needle in NEEDLES {
        let mut from = 0;
        while let Some(found) = text[from..].find(needle.text) {
            let offset = from + found;
            // `os.environ` occurs inside `myos.environ`, and `env::var(` inside
            // `myenv::var(`. A call starts where an identifier starts.
            if starts_an_identifier(text, offset) {
                let rest = &text[offset + needle.text.len()..];
                if let Some(taken) = (needle.take)(rest) {
                    let end = offset + needle.text.len() + taken.used;
                    if best.as_ref().is_none_or(|(at, _, _, _)| offset < *at) {
                        best = Some((offset, taken.form, taken.name, end));
                    }
                    break;
                }
            }
            from = offset + needle.text.len();
        }
    }
    best
}

/// Whether a needle at `offset` begins a word rather than sitting inside one.
fn starts_an_identifier(text: &str, offset: usize) -> bool {
    match text[..offset].chars().next_back() {
        None => true,
        Some(before) => !(before.is_alphanumeric() || before == '_' || before == '$'),
    }
}

/// One needle, and how to take the key that follows it.
struct Needle {
    text: &'static str,
    take: fn(&str) -> Option<Taken>,
}

/// A read a needle's parser found: which form, which key, and how much of the
/// text after the needle it used.
///
/// The form comes back from the parser rather than from the needle, because
/// `process.env.NAME` and `process.env["NAME"]` share one needle and are two
/// forms. A needle that carried the form would report every computed read as a
/// property read — which it did, until `the_keys_of_the_recognised_forms_are_the_keys_a_person_wrote`
/// caught it.
struct Taken {
    form: ReadForm,
    name: String,
    used: usize,
}

/// Every needle, and the parser for the key that follows it.
///
/// **No needle is a prefix of another**, which is what makes the order of this
/// table not matter. `env::var(` and `env::var_os(` differ at the eighth
/// character, and the others share no prefix at all;
/// `no_needle_hides_another` holds that, so that a needle added later cannot
/// silently make an existing one unreachable.
const NEEDLES: &[Needle] = &[
    Needle {
        text: "process.env",
        take: take_node_env,
    },
    Needle {
        text: "os.environ",
        take: take_python_environ,
    },
    Needle {
        text: "os.getenv",
        take: take_python_getenv,
    },
    Needle {
        text: "env::var_os(",
        take: take_rust_env,
    },
    Needle {
        text: "env::var(",
        take: take_rust_env,
    },
];

/// `process.env.NAME` or `process.env["NAME"]`.
fn take_node_env(rest: &str) -> Option<Taken> {
    if let Some(after) = rest.strip_prefix('.') {
        let (name, used) = take_identifier(after)?;
        return Some(Taken {
            form: ReadForm::NodeProperty,
            name,
            used: 1 + used,
        });
    }
    let (name, used) = take_group(rest)?;
    Some(Taken {
        form: ReadForm::NodeComputed,
        name,
        used,
    })
}

/// `os.environ["NAME"]` or `os.environ.get("NAME")`.
fn take_python_environ(rest: &str) -> Option<Taken> {
    // `.get(` and not `.get`, so that `os.environ.getting` is read as the key
    // `getting` rather than as a call whose argument is a syntax error.
    let (name, used) = match rest.strip_prefix(".get(") {
        Some(after) => {
            let (name, used) = take_quoted(after)?;
            (name, 5 + used)
        }
        None => take_group(rest)?,
    };
    Some(Taken {
        form: ReadForm::PythonEnviron,
        name,
        used,
    })
}

/// `os.getenv("NAME")`.
fn take_python_getenv(rest: &str) -> Option<Taken> {
    let (name, used) = take_group(rest)?;
    Some(Taken {
        form: ReadForm::PythonGetenv,
        name,
        used,
    })
}

/// `env::var("NAME")` and the `var_os` spelling, with the `(` already consumed
/// by the needle.
fn take_rust_env(rest: &str) -> Option<Taken> {
    let (name, used) = take_quoted(rest)?;
    Some(Taken {
        form: ReadForm::RustEnv,
        name,
        used,
    })
}

/// A bracketed or called read: `["NAME"]`, `('NAME')`.
fn take_group(rest: &str) -> Option<(String, usize)> {
    let opener = rest.chars().next()?;
    if opener != '[' && opener != '(' {
        return None;
    }
    let (name, used) = take_quoted(&rest[1..])?;
    Some((name, 1 + used))
}

/// A quoted argument: `"NAME"`, `'NAME'`, ` "NAME"` — with the brackets already
/// consumed by the caller.
fn take_quoted(rest: &str) -> Option<(String, usize)> {
    let stripped = rest.trim_start();
    let spaces = rest.len() - stripped.len();
    let quote = stripped.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &stripped[1..];
    let end = body.find(quote)?;
    let name = &body[..end];
    if !is_key(name) {
        return None;
    }
    // What follows the closing quote decides whether this was a key at all. A
    // `+`, a `%` or a `,`-less continuation means the argument was built rather
    // than written, and a key SURE assembled is not a key the project stated.
    let after = body[end + 1..].trim_start();
    if !after.starts_with([']', ')', ',']) {
        return None;
    }
    Some((name.to_owned(), spaces + 1 + end + 1))
}

/// A bare identifier after a dot.
fn take_identifier(rest: &str) -> Option<(String, usize)> {
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if !is_key(name) {
        return None;
    }
    Some((name.to_owned(), end))
}

/// Whether text is a key rather than something a key was built out of.
///
/// Deliberately loose about case: environment keys have no case rule, and a key
/// rejected for its case would be a read SURE did not report.
fn is_key(text: &str) -> bool {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The key an `.env`-style line declares, or `None`.
///
/// Takes the text before the first `=` and nothing after it. A commented-out
/// line counts, because a template comments out the keys it does not set by
/// default and the key is still declared; `#` is therefore stripped rather than
/// treated as a reason to skip the line.
///
/// The signature takes one `&str` and returns an owned key with no room for the
/// right-hand side. That is this module's rule expressed in a type: there is
/// nowhere for a value to go.
#[must_use]
pub fn key_from_env_line(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    for prefix in ["export", "set"] {
        if let Some(after) = following_word(rest, prefix) {
            rest = after.trim_start();
        }
    }
    for comment in ['#'] {
        if let Some(after) = rest.strip_prefix(comment) {
            rest = after.trim_start();
        }
    }
    if let Some(after) = rest.strip_prefix("//") {
        rest = after.trim_start();
    }
    let (name, used) = take_identifier(rest)?;
    if !rest[used..].trim_start().starts_with('=') {
        return None;
    }
    Some(name)
}

/// `rest` without a leading `word`, when `word` is a whole word.
///
/// `exported=1` must not be read as `export` followed by `ed=1`, which would
/// declare the key `ed`.
fn following_word<'a>(rest: &'a str, word: &str) -> Option<&'a str> {
    let after = rest.strip_prefix(word)?;
    match after.chars().next() {
        Some(c) if c.is_whitespace() => Some(after),
        _ => None,
    }
}

/// Every key a documentation line declares, in the order they appear.
///
/// The rule is an **assignment**: an upper-case identifier followed by `=`, as
/// in a fenced example (`export DATABASE_URL=…`) or a line of prose that
/// documents a setting by writing it out. Whitespace before the `=` is allowed,
/// because `PORT = 3000` is a settings line rather than a different thing.
///
/// A key that is only ever *mentioned* — in a table, in a sentence, in a code
/// span such as `` `PORT` `` — is not found, and that is a stated limit rather
/// than an oversight. The alternative was matching any upper-case word in
/// backticks, which finds `README`, `TODO` and `JSON`, and a declaration list
/// full of those is one nobody reads. This way the list is short and every entry
/// in it is a line somebody wrote a value beside.
///
/// The right-hand side is not examined, so a documented value is never read —
/// only the key before the `=` is.
#[must_use]
pub fn keys_from_setting_line(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut found: Vec<String> = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_uppercase() || byte == b'_') {
            index += 1;
            continue;
        }
        let start = index;
        let mut end = index;
        while end < bytes.len()
            && (bytes[end].is_ascii_uppercase()
                || bytes[end].is_ascii_digit()
                || bytes[end] == b'_')
        {
            end += 1;
        }
        let name = &line[start..end];
        // The character before has to be one an identifier cannot continue from,
        // so that `xDATABASE_URL=` is not a declaration of `DATABASE_URL`.
        let before_is_word = line[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$');
        // Whitespace between the key and the `=` is normal in a `.env` file and
        // in a settings block, so `PORT = 3000` is the same declaration as
        // `PORT=3000` and neither is a declaration of `PORT\ `.
        let mut equals = end;
        while matches!(bytes.get(equals), Some(b' ' | b'\t')) {
            equals += 1;
        }
        // A single `=` and not `==`, `=>`, `>=` or `!=`.
        let is_assignment = bytes.get(equals) == Some(&b'=')
            && bytes.get(equals + 1) != Some(&b'=')
            && bytes.get(equals + 1) != Some(&b'>');
        if !before_is_word && is_assignment && name.len() >= 3 && !found.iter().any(|k| k == name) {
            found.push(name.to_owned());
        }
        index = end.max(index + 1);
    }
    found
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_taken_and_the_value_is_left_behind() {
        // The line a `.env.example` has in it, with a value that must not
        // survive the call.
        let key = key_from_env_line("API_KEY=sk-live-canary-0000").unwrap();
        assert_eq!(key, "API_KEY");
        assert!(!key.contains("sk-live"));
        assert_eq!(
            key_from_env_line("export DATABASE_URL=postgres://u:p@h/d").unwrap(),
            "DATABASE_URL"
        );
        assert_eq!(key_from_env_line("  TOKEN =  ").unwrap(), "TOKEN");
        assert_eq!(
            key_from_env_line("# OPTIONAL_KEY=").unwrap(),
            "OPTIONAL_KEY"
        );
        assert_eq!(key_from_env_line("// PASTED_KEY=1").unwrap(), "PASTED_KEY");
    }

    #[test]
    fn a_word_that_only_starts_like_a_prefix_is_not_a_prefix() {
        // `exported=1` is the key `exported`, not `export` followed by `ed`.
        assert_eq!(key_from_env_line("exported=1").unwrap(), "exported");
        assert_eq!(key_from_env_line("settings=1").unwrap(), "settings");
    }

    #[test]
    fn a_line_that_declares_nothing_is_not_a_declaration() {
        assert_eq!(key_from_env_line("# just a comment"), None);
        assert_eq!(key_from_env_line(""), None);
        assert_eq!(key_from_env_line("   "), None);
        assert_eq!(key_from_env_line("=novalue"), None);
        // A shell-style reference is a use of a key, not a declaration of one.
        assert_eq!(key_from_env_line("$API_KEY"), None);
        assert_eq!(key_from_env_line("${API_KEY}"), None);
    }

    #[test]
    fn a_template_is_recognised_and_the_file_the_values_are_in_is_not() {
        for name in [
            ".env.example",
            ".env.sample",
            ".env.template",
            ".env.dist",
            ".env.defaults",
            ".env.local.example",
            ".env.example.local",
            ".ENV.EXAMPLE",
        ] {
            assert!(is_env_template(name), "{name} should be a template");
        }
        for name in [
            ".env",
            ".env.local",
            ".env.production",
            ".env.development",
            ".env.test",
            ".envrc",
            "env.example",
            "example.env",
            ".environment",
        ] {
            assert!(!is_env_template(name), "{name} should not be a template");
        }
    }

    #[test]
    fn the_file_where_the_values_are_is_never_a_candidate() {
        for name in [".env", ".env.local", ".env.production", "app/.env"] {
            assert_eq!(declaration_candidate(Path::new(name)), None, "{name}");
        }
        assert_eq!(
            declaration_candidate(Path::new("app/.env.example")),
            Some(DeclarationKind::Example)
        );
        assert_eq!(
            declaration_candidate(Path::new("README.md")),
            Some(DeclarationKind::Document)
        );
        assert_eq!(
            declaration_candidate(Path::new("docs/setup.md")),
            Some(DeclarationKind::Document)
        );
        assert_eq!(declaration_candidate(Path::new("sure.yaml")), None);
    }

    #[test]
    fn a_source_file_is_recognised_by_its_extension_and_nothing_else() {
        for name in [
            "a.js", "a.ts", "a.tsx", "a.mjs", "a.cjs", "a.py", "a.pyi", "a.rs", "A.RS",
        ] {
            assert!(is_source_candidate(Path::new(name)), "{name}");
        }
        for name in [
            "a.md",
            "a.txt",
            "a.json",
            "a",
            "a.rs.bak",
            ".env.example",
            "a.jsx.txt",
        ] {
            assert!(!is_source_candidate(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn a_document_that_assigns_a_key_declares_it_and_one_that_mentions_it_does_not() {
        assert_eq!(
            keys_from_setting_line("export DATABASE_URL=postgres://localhost/db"),
            ["DATABASE_URL"]
        );
        assert_eq!(keys_from_setting_line("PORT=3000"), ["PORT"]);
        assert_eq!(
            keys_from_setting_line("    API_BASE_URL = x"),
            ["API_BASE_URL"]
        );
        // Several on one line, which is what a shell example looks like.
        assert_eq!(
            keys_from_setting_line("SMTP_HOST=mail.example SMTP_PORT=587"),
            ["SMTP_HOST", "SMTP_PORT"]
        );
        // A mention is not a declaration — the stated limit.
        assert_eq!(
            keys_from_setting_line("Set `PORT` to change it."),
            Vec::<String>::new()
        );
        assert_eq!(
            keys_from_setting_line("| `DATABASE_URL` | the database |"),
            Vec::<String>::new()
        );
        // Nor is something that merely looks like one.
        assert_eq!(keys_from_setting_line("README"), Vec::<String>::new());
        assert_eq!(keys_from_setting_line("if a == b"), Vec::<String>::new());
        assert_eq!(
            keys_from_setting_line("xDATABASE_URL=1"),
            Vec::<String>::new()
        );
        assert_eq!(
            keys_from_setting_line("CHECK => value"),
            Vec::<String>::new()
        );
        // An upper-case word too short to be a key.
        assert_eq!(keys_from_setting_line("OK=1"), Vec::<String>::new());
    }

    #[test]
    fn a_documented_key_is_taken_and_the_value_beside_it_is_not() {
        let keys = keys_from_setting_line("POSTGRES_PASSWORD=hunter2-canary");
        assert_eq!(keys, ["POSTGRES_PASSWORD"]);
        assert!(!format!("{keys:?}").contains("hunter2"));
    }

    #[test]
    fn each_read_form_names_the_ecosystem_it_belongs_to() {
        assert_eq!(ReadForm::NodeProperty.ecosystem(), Ecosystem::Node);
        assert_eq!(ReadForm::NodeComputed.ecosystem(), Ecosystem::Node);
        assert_eq!(ReadForm::PythonEnviron.ecosystem(), Ecosystem::Python);
        assert_eq!(ReadForm::PythonGetenv.ecosystem(), Ecosystem::Python);
        assert_eq!(ReadForm::RustEnv.ecosystem(), Ecosystem::Rust);
        assert_eq!(ReadForm::ALL.len(), 5);
    }

    #[test]
    fn every_form_the_table_lists_can_be_produced_by_a_line() {
        // A form in `ALL` that no needle produces would be a form a report
        // claims to look for and does not.
        let produced: Vec<ReadForm> = [
            "const a = process.env.NAME;",
            "const b = process.env[\"NAME\"];",
            "v = os.environ[\"NAME\"]",
            "v = os.getenv(\"NAME\")",
            "let v = env::var(\"NAME\");",
        ]
        .iter()
        .map(|line| next_read(line).expect("a read").1)
        .collect();
        for form in ReadForm::ALL {
            assert!(produced.contains(form), "no needle produces {form:?}");
        }
    }

    #[test]
    fn no_needle_hides_another() {
        // A needle that is a prefix of another would take the longer one's
        // matches if it were listed first, and the table's order would start to
        // matter silently.
        for (index, outer) in NEEDLES.iter().enumerate() {
            for (other, inner) in NEEDLES.iter().enumerate() {
                if index == other {
                    continue;
                }
                assert!(
                    !outer.text.starts_with(inner.text),
                    "{} is a prefix of {}",
                    inner.text,
                    outer.text
                );
                assert_ne!(outer.text, inner.text, "two needles are the same text");
            }
        }
    }

    #[test]
    fn the_keys_of_the_recognised_forms_are_the_keys_a_person_wrote() {
        let cases: &[(&str, ReadForm, &str)] = &[
            (
                "const k = process.env.DATABASE_URL;",
                ReadForm::NodeProperty,
                "DATABASE_URL",
            ),
            (
                "const k = process.env['DATABASE_URL'];",
                ReadForm::NodeComputed,
                "DATABASE_URL",
            ),
            (
                "const k = process.env[\"DATABASE_URL\"]",
                ReadForm::NodeComputed,
                "DATABASE_URL",
            ),
            (
                "k = os.environ['DATABASE_URL']",
                ReadForm::PythonEnviron,
                "DATABASE_URL",
            ),
            (
                "k = os.environ.get(\"DATABASE_URL\")",
                ReadForm::PythonEnviron,
                "DATABASE_URL",
            ),
            (
                "k = os.environ.get( \"DATABASE_URL\" , None)",
                ReadForm::PythonEnviron,
                "DATABASE_URL",
            ),
            (
                "k = os.getenv('DATABASE_URL')",
                ReadForm::PythonGetenv,
                "DATABASE_URL",
            ),
            (
                "let k = std::env::var(\"DATABASE_URL\")?;",
                ReadForm::RustEnv,
                "DATABASE_URL",
            ),
            (
                "let k = env::var_os(\"DATABASE_URL\");",
                ReadForm::RustEnv,
                "DATABASE_URL",
            ),
            (
                "let k = std::env::var_os('DATABASE_URL').is_none()",
                ReadForm::RustEnv,
                "DATABASE_URL",
            ),
            ("k = process.env.getting", ReadForm::NodeProperty, "getting"),
        ];
        for &(line, form, name) in cases {
            let (_, found_form, found_name, _) =
                next_read(line).unwrap_or_else(|| panic!("no read found in {line:?}"));
            assert_eq!(found_form, form, "{line:?}");
            assert_eq!(found_name, name, "{line:?}");
        }
    }

    #[test]
    fn a_key_the_code_built_at_run_time_is_not_a_read() {
        for line in [
            "const k = process.env[which];",
            "const k = process.env[name + \"_URL\"];",
            "const k = process.env[`${prefix}_URL`];",
            "k = os.getenv(name)",
            "k = os.environ[prefix + \"_URL\"]",
            "let k = env::var(&name);",
            "let k = env::var(format!(\"{prefix}_URL\"));",
        ] {
            assert!(
                next_read(line).is_none(),
                "{line:?} should not be a read SURE claims to have found"
            );
        }
    }

    #[test]
    fn a_key_written_beside_another_piece_of_text_is_not_a_key() {
        // The closing quote is followed by a continuation, so what the project
        // states is the literal plus whatever the rest holds — a key SURE would
        // have to assemble, and so not a key the project stated. With the space
        // this is decided by the space; without one, by the `+` itself. Both
        // spellings are here because a rule that only rejects the spaced form
        // misses whichever of the two a project happens to write.
        for line in [
            "const k = process.env[\"API_\" + SUFFIX];",
            "const k = process.env[\"API_\"+SUFFIX];",
            "k = os.environ['API_'+SUFFIX]",
            "let k = env::var(\"API_\".to_owned() + SUFFIX);",
        ] {
            assert!(
                next_read(line).is_none(),
                "{line:?} should not be a read SURE claims to have found"
            );
        }
    }

    #[test]
    fn a_quoted_argument_that_is_not_a_key_is_not_a_read() {
        // Quoting an argument does not make it a name. SURE records a key only
        // when what is inside the quotes could be one, because a report listing
        // `not a key` invites a person to go and look for it.
        for line in [
            "const k = process.env[\"\"];",
            "const k = process.env[\"not a key\"];",
            "const k = process.env[\"1BAD\"];",
            "const k = process.env[\"has-dash\"];",
            "k = os.environ['']",
        ] {
            assert!(
                next_read(line).is_none(),
                "{line:?} should not be a read SURE claims to have found"
            );
        }
    }

    #[test]
    fn a_needle_inside_a_longer_identifier_is_not_a_needle() {
        for line in [
            "k = myos.environ[\"NAME\"]",
            "k = await os.getenvx(\"NAME\")",
            "let k = myenv::var(\"NAME\");",
            "k = xos.getenv(\"NAME\")",
        ] {
            assert!(next_read(line).is_none(), "{line:?}");
        }
    }

    #[test]
    fn every_read_on_a_line_is_found_and_not_only_the_first() {
        let line = "const a = process.env.FIRST, b = process.env.SECOND, c = process.env.THIRD;";
        let found = references_in(line, Path::new("a.js"));
        let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["FIRST", "SECOND", "THIRD"]);
        assert!(found.iter().all(|r| r.line == 1));
    }

    #[test]
    fn a_line_number_is_the_line_the_read_is_on() {
        let text = "const a = 1;\n\nconst b = process.env.ON_LINE_THREE;\n";
        let found = references_in(text, Path::new("a.js"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn a_carriage_return_is_not_part_of_the_line() {
        let found = references_in("const a = process.env.NAME;\r\n", Path::new("a.js"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "NAME");
    }

    #[test]
    fn a_mention_inside_a_comment_is_still_a_read() {
        // Pinned rather than fixed. This module matches text, so a key named in
        // a comment counts as a key the project asks for. The alternative is a
        // parser per language, which is different work with a different failure
        // mode: a parser that got a language wrong would drop real reads
        // *silently*, and this way the limit is one a reader meets in the module
        // documentation and in this test.
        let found = references_in("// TODO: drop process.env.OLD_NAME\n", Path::new("a.js"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "OLD_NAME");
    }

    #[test]
    fn every_status_says_something_a_person_could_read() {
        for status in KeyStatus::ALL {
            let sentence = status.plain_description();
            assert!(sentence.ends_with('.'), "{status:?}");
            assert!(
                sentence.chars().next().is_some_and(char::is_uppercase),
                "{status:?}"
            );
            assert!(!sentence.contains('_'), "{status:?} leaks its wire name");
        }
    }

    #[test]
    fn the_two_one_sided_statuses_do_not_claim_the_project_has_nothing() {
        // The wording rule, held as a test: neither arm may say a key is
        // "missing", because without asking `is_complete` the strongest true
        // statement is about the two lists.
        for status in [KeyStatus::ReadButNotDeclared, KeyStatus::DeclaredButNotRead] {
            let sentence = status.plain_description().to_lowercase();
            for word in [
                "missing",
                "absent",
                "undefined",
                "forgot",
                "never",
                "should",
            ] {
                assert!(!sentence.contains(word), "{status:?} says {word:?}");
            }
            assert!(sentence.contains("that sure read"), "{status:?}");
        }
    }

    #[test]
    fn every_reason_says_what_it_means_without_reading_the_fields() {
        let reasons = [
            UnreadReason::OutOfBudget { limit: 1 },
            UnreadReason::OutOfBytes { limit: 1 },
            UnreadReason::TooLarge { limit: 1 },
            UnreadReason::Unreadable {
                detail: "denied".to_owned(),
            },
            UnreadReason::NotText {
                detail: "invalid".to_owned(),
            },
        ];
        for reason in &reasons {
            let sentence = reason.plain_description();
            assert!(!sentence.is_empty(), "{reason:?}");
            assert!(!sentence.contains("limit"), "{reason:?}");
        }
    }

    #[test]
    fn every_read_form_has_a_stable_name_of_its_own() {
        for (index, form) in ReadForm::ALL.iter().enumerate() {
            assert!(!form.as_str().is_empty());
            assert!(!form.as_str().ends_with('.'), "{form:?}");
            assert!(!form.plain_description().is_empty(), "{form:?}");
            assert!(!form.plain_description().ends_with('.'), "{form:?}");
            // A name no other form shares, so a report cannot be ambiguous.
            for other in &ReadForm::ALL[index + 1..] {
                assert_ne!(form.as_str(), other.as_str());
                assert_ne!(form.plain_description(), other.plain_description());
            }
        }
    }

    #[test]
    fn the_options_are_the_defaults_until_a_caller_changes_one() {
        let options = ReferenceOptions::default();
        assert_eq!(
            options,
            ReferenceOptions::default().with_max_files(options.max_files)
        );
        assert_eq!(
            ReferenceOptions::default()
                .with_max_file_bytes(7)
                .max_file_bytes,
            7
        );
        assert_eq!(ReferenceOptions::default().with_max_files(3).max_files, 3);
        assert_eq!(
            ReferenceOptions::default()
                .with_max_total_bytes(9)
                .max_total_bytes,
            9
        );
    }
}

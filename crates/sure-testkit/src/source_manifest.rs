//! `SHA256SUMS.txt`: what it describes, and the one form in which it can be true.
//!
//! `P15-T020`'s acceptance is *"Give the checksum manifest a reader, or take it
//! out of the tree."* The file stayed, so this module is the reader, and the
//! decision it encodes is **which bytes** the manifest describes.
//!
//! # The premise, measured rather than assumed
//!
//! Before this module existed, nothing in the repository read the manifest. The
//! only thing that maintained it was a scratch script under `target/tmp`, which
//! is git-ignored, and both bootstrap gates exit 0 over a manifest whose digests
//! are wrong — so the file could be arbitrarily stale and every gate this
//! repository runs would still say the tree is fine.
//!
//! # Which bytes, and why it is not a detail
//!
//! `.gitattributes` carries `* text=auto eol=lf` with `*.ps1 text eol=crlf`, so
//! a path's bytes in the **index** and its bytes in the **working tree** are two
//! different things, and for the ten `.ps1` paths this manifest lists they are
//! different lengths. A check that picks the wrong one is not slightly wrong: it
//! reports files as stale when nothing has changed.
//!
//! Measured on 2026-09-21 at `edb2b00`, on a fresh `git clone` of this
//! repository, over the 195 paths the manifest listed at that commit:
//!
//! - a reader comparing against **working-tree** bytes reports **8 stale** —
//!   `integrations/claude-code/scripts/sure-hook.ps1`,
//!   `integrations/cursor/scripts/sure-hook.ps1`, `scripts/Bootstrap-Sure.ps1`,
//!   `scripts/Install-DevDeps-Windows.ps1`, `scripts/Preflight-Windows.ps1`,
//!   `scripts/Publish-Bootstrap.ps1`, `scripts/Test-SureEnvironment.ps1` and
//!   `scripts/Validate-Bootstrap.ps1` — because a fresh checkout applies
//!   `eol=crlf` and writes CRLF, while the long-lived worktree the manifest was
//!   built in holds LF for those eight;
//! - a reader comparing against **index** bytes reports **2 stale** — the
//!   `integrations/agent-plugin/scripts/*.ps1` pair, whose recorded digests are
//!   the only two in the file written from a checkout rather than from the index.
//!
//! So neither form matched all ten, because the manifest as it stood was built
//! from whichever bytes happened to be on disk at the time and was therefore
//! internally inconsistent. The two forms also differ in kind: the working-tree
//! form is a fact about a *machine* — it changes with a checkout, with an
//! editor's line endings, and with whether a path was materialised at all —
//! while the index form is a fact about the *commit*, and is byte-identical on
//! Windows, macOS and Linux because the `eol` attributes are applied on
//! checkout and never to the object.
//!
//! This module therefore reads [`Index`] bytes, and the manifest was regenerated
//! from the index in the same commit that added this module. That is what makes
//! the check green on a correct tree on all three platforms and in any clone,
//! which is the property that decides it: a check that is red on every fresh
//! clone would be worse than the silence it replaces, and reporting two
//! mismatches that are not staleness is the failure the acceptance clause about
//! `.gitattributes` names rather than leaves to be discovered.
//!
//! # What it does not cover
//!
//! [`MANIFEST`] does not list itself, and cannot: a file cannot contain its own
//! digest, because writing the digest changes the content that the digest is of.
//! The manifest's own bytes are covered by git instead — the object the commit
//! stores is what a clone materialises, and `git fsck` is what verifies it.
//! [`parse`] therefore has nothing to say about self-reference, and
//! `the_manifest_does_not_list_itself` in `tests/source_manifest.rs` holds the
//! exclusion so that a later contributor meets the reason rather than the
//! regress.
//!
//! # The two ways this could be silently useless
//!
//! A checker that read an index it could not find would report every path as
//! absent rather than as present-and-correct, and a checker with no entries to
//! check would report nothing wrong with anything. [`parse`] refuses a manifest
//! with no entries, and `the_check_reads_the_index_blob_and_not_the_working_tree`
//! drives the checker with bytes a test chose so that a reader which quietly
//! fell back to the checkout cannot pass.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{BufRead as _, BufReader, BufWriter, Read as _, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};

use sha2::{Digest as _, Sha256};

/// The file this module is about, as the repository root spells it.
///
/// Also the name the release pipeline must **not** attach: `release.yml` and
/// `scripts/Assemble-Release.sh` both say so, and
/// `crates/sure-testkit/tests/ci_workflow.rs` holds that rule.
pub const MANIFEST: &str = "SHA256SUMS.txt";

/// The program whose index is read. Named here rather than spelled at each call
/// site so that a message about it is about the same thing everywhere.
const GIT: &str = "git";

/// One `<digest>  <path>` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The recorded SHA-256, lowercase hexadecimal, 64 characters.
    pub digest: String,
    /// The path, as written — repository-relative and forward-slashed.
    pub path: String,
}

/// A parsed manifest, in the order it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The entries, in file order.
    pub entries: Vec<Entry>,
    /// Whether the file ended with a newline.
    ///
    /// Kept because the committed file does not, and a regeneration that
    /// silently added one would be a change nobody asked for in a file whose
    /// whole subject is whether two things are the same.
    pub trailing_newline: bool,
}

/// Why a manifest could not be read as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A non-blank line that is not `<digest>  <path>`.
    NotAnEntry {
        /// 1-based line number.
        line: usize,
        /// The line, as written.
        text: String,
    },
    /// A digest field that is not 64 lowercase hexadecimal characters.
    NotADigest {
        /// 1-based line number.
        line: usize,
        /// The field, as written.
        digest: String,
    },
    /// A line whose path field is empty.
    NoPath {
        /// 1-based line number.
        line: usize,
    },
    /// A manifest with nothing in it.
    ///
    /// An error rather than an empty result, and this is the whole reason the
    /// variant exists: a checker handed nothing to check would find nothing
    /// wrong and report success, which is the false green this file is about.
    NoEntries,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnEntry { line, text } => write!(
                f,
                "line {line} is not `<64-hex>  <path>`: {}",
                shorten(text)
            ),
            Self::NotADigest { line, digest } => write!(
                f,
                "line {line} has a digest that is not 64 lowercase hexadecimal characters: {}",
                shorten(digest)
            ),
            Self::NoPath { line } => write!(f, "line {line} has an empty path"),
            Self::NoEntries => write!(
                f,
                "the manifest lists nothing, so checking it would verify nothing"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// A bounded excerpt, so a message about a 2000-character line stays readable.
fn shorten(text: &str) -> String {
    const LIMIT: usize = 80;
    if text.chars().count() <= LIMIT {
        return text.to_owned();
    }
    let head: String = text.chars().take(LIMIT).collect();
    format!("{head}…")
}

/// Parse the manifest.
///
/// Blank lines are skipped rather than rejected, because a trailing newline is
/// one and the committed file has not got one but a hand-edit might add it.
pub fn parse(text: &str) -> Result<Manifest, ParseError> {
    let mut entries = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let Some((digest, path)) = line.split_once("  ") else {
            return Err(ParseError::NotAnEntry {
                line: number,
                text: line.to_owned(),
            });
        };
        let well_formed = digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        if !well_formed {
            return Err(ParseError::NotADigest {
                line: number,
                digest: digest.to_owned(),
            });
        }
        if path.is_empty() {
            return Err(ParseError::NoPath { line: number });
        }
        entries.push(Entry {
            digest: digest.to_owned(),
            path: path.to_owned(),
        });
    }
    if entries.is_empty() {
        return Err(ParseError::NoEntries);
    }
    Ok(Manifest {
        entries,
        trailing_newline: text.ends_with('\n'),
    })
}

/// Write a manifest back out, byte-for-byte the way [`parse`] read it.
#[must_use]
pub fn render(manifest: &Manifest) -> String {
    let mut text = manifest
        .entries
        .iter()
        .map(|entry| format!("{}  {}", entry.digest, entry.path))
        .collect::<Vec<_>>()
        .join("\n");
    if manifest.trailing_newline {
        text.push('\n');
    }
    text
}

/// A byte string's SHA-256, lowercase hexadecimal.
#[must_use]
pub fn digest_of(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

/// Bytes as lowercase hexadecimal.
///
/// Written out rather than formatted per byte so that a digest is always the
/// same 64 characters: a 63-character digest would compare unequal to itself,
/// and everything here is string comparison.
fn hex(bytes: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut text = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// Why the index could not be read.
#[derive(Debug)]
pub enum IndexError {
    /// `git` is not runnable, which is a broken checkout rather than a finding.
    GitUnavailable {
        /// The repository root `git` was asked about.
        root: std::path::PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A `git` command failed.
    GitFailed {
        /// The command line, as typed.
        command: String,
        /// The status it exited with, if it exited.
        status: Option<i32>,
        /// What it wrote to stderr.
        stderr: String,
    },
    /// An index record this reader does not understand.
    ///
    /// Reported rather than skipped: skipping would leave a listed path looking
    /// absent from the index, which is a finding about the manifest.
    UnreadableIndexRecord {
        /// The record, as bytes, shortened for display.
        record: String,
    },
    /// A listed object that is not a blob.
    NotABlob {
        /// The path.
        path: String,
        /// The object type `git` reported.
        kind: String,
    },
    /// A read from `git`'s output failed.
    Io(std::io::Error),
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitUnavailable { root, source } => write!(
                f,
                "`{GIT}` must be runnable to read the index at {}: {source}; without it the \
                 manifest cannot be checked at all",
                root.display()
            ),
            Self::GitFailed {
                command,
                status,
                stderr,
            } => write!(
                f,
                "`{command}` exited {status:?}: {}",
                stderr.trim_end().lines().next().unwrap_or("")
            ),
            Self::UnreadableIndexRecord { record } => {
                write!(f, "unreadable index record: {}", shorten(record))
            }
            Self::NotABlob { path, kind } => write!(
                f,
                "{path} is a {kind} in the index, not a blob, so there are no bytes to hash"
            ),
            Self::Io(source) => write!(f, "reading from `{GIT}` failed: {source}"),
        }
    }
}

impl std::error::Error for IndexError {}

/// The bytes the **index** holds for the paths the manifest lists.
///
/// This is the form the manifest describes. See the module comment for why it is
/// this one and not the working tree, and for the measurement that decided it.
#[derive(Debug, Clone, Default)]
pub struct Index {
    blobs: BTreeMap<String, Vec<u8>>,
}

impl Index {
    /// An index from blobs that were read some other way.
    ///
    /// Exists so the checker can be driven with bytes a test chose, which is
    /// how a rule about *which* bytes are hashed is held without depending on
    /// what this machine's checkout happens to contain.
    #[must_use]
    pub fn from_blobs(blobs: BTreeMap<String, Vec<u8>>) -> Self {
        Self { blobs }
    }

    /// Read the index at `root`, keeping only the `wanted` paths.
    pub fn read(root: &Path, wanted: &[String]) -> Result<Self, IndexError> {
        let wanted: BTreeSet<&str> = wanted.iter().map(String::as_str).collect();
        let objects = list_index_objects(root, &wanted)?;
        let blobs = read_blobs(root, &objects)?;
        Ok(Self { blobs })
    }

    /// The blob for one path, if the index holds one.
    #[must_use]
    pub fn blob(&self, path: &str) -> Option<&[u8]> {
        self.blobs.get(path).map(Vec::as_slice)
    }

    /// How many paths the index was read for.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    /// Whether the index holds nothing, which is never a pass.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }
}

/// Run a `git` command in `root` and return its stdout.
fn git_output(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, IndexError> {
    let command = format!("{GIT} {}", arguments.join(" "));
    let output = Command::new(GIT)
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|source| IndexError::GitUnavailable {
            root: root.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(IndexError::GitFailed {
            command,
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(output.stdout)
}

/// `path -> object id` for every wanted path at index stage 0.
fn list_index_objects(
    root: &Path,
    wanted: &BTreeSet<&str>,
) -> Result<BTreeMap<String, String>, IndexError> {
    // `-z`, so a path with a space, a quote or a newline in it arrives as itself
    // rather than in `git`'s core.quotePath spelling. CLAUDE.md asks for exactly
    // this: this repository is developed on Windows and its files have spaces.
    let listing = git_output(root, &["ls-files", "--stage", "-z"])?;
    let mut objects = BTreeMap::new();
    for record in listing.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return Err(IndexError::UnreadableIndexRecord {
                record: String::from_utf8_lossy(record).into_owned(),
            });
        };
        let (metadata, path) = record.split_at(tab);
        let metadata =
            std::str::from_utf8(metadata).map_err(|_| IndexError::UnreadableIndexRecord {
                record: String::from_utf8_lossy(record).into_owned(),
            })?;
        // `<mode> <object> <stage>`.
        let mut fields = metadata.split(' ');
        let _mode = fields.next();
        let object = fields.next();
        let stage = fields.next();
        let (Some(object), Some("0")) = (object, stage) else {
            // An unmerged path has stages 1..3 and no stage 0, and it is not a
            // thing the manifest can describe. The paths it wants are looked up
            // by name below, so a path that is only unmerged stays absent from
            // `objects` and is reported as a finding about the manifest rather
            // than quietly resolved to one of its sides.
            continue;
        };
        // A path that is not valid UTF-8 cannot be named by a UTF-8 manifest, so
        // it can never be one of the wanted paths.
        let Ok(path) = std::str::from_utf8(&path[1..]) else {
            continue;
        };
        if wanted.contains(path) {
            objects.insert(path.to_owned(), object.to_owned());
        }
    }
    Ok(objects)
}

/// Read the contents of a set of objects, in one `git cat-file` process.
///
/// One process rather than one per path: the manifest has ~195 entries, and a
/// process per entry is two hundred process creations on every `cargo test` and
/// on every platform of the matrix. The protocol is `--batch`'s — write an
/// object id, flush, read `<id> <type> <size>\n<size bytes>\n` — and it is
/// written as a strict alternation rather than as "write all, then read all",
/// because the child blocks writing to a full pipe when its output outgrows the
/// buffer, so writing everything first would deadlock on any real manifest.
fn read_blobs(
    root: &Path,
    objects: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<u8>>, IndexError> {
    if objects.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut child = Command::new(GIT)
        .current_dir(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| IndexError::GitUnavailable {
            root: root.to_path_buf(),
            source,
        })?;

    let mut to_git = BufWriter::new(child.stdin.take().ok_or_else(|| IndexError::GitFailed {
        command: format!("{GIT} cat-file --batch"),
        status: None,
        stderr: "no stdin".to_owned(),
    })?);
    let mut from_git =
        BufReader::new(child.stdout.take().ok_or_else(|| IndexError::GitFailed {
            command: format!("{GIT} cat-file --batch"),
            status: None,
            stderr: "no stdout".to_owned(),
        })?);

    let mut blobs = BTreeMap::new();
    for (path, object) in objects {
        writeln!(to_git, "{object}").map_err(IndexError::Io)?;
        to_git.flush().map_err(IndexError::Io)?;

        let mut header = String::new();
        from_git.read_line(&mut header).map_err(IndexError::Io)?;
        let mut fields = header.split(' ');
        let _id = fields.next();
        let kind = fields.next().unwrap_or_default().to_owned();
        let size = fields.next().map(str::trim_end).unwrap_or_default();
        if kind != "blob" {
            return Err(IndexError::NotABlob {
                path: path.clone(),
                kind: if kind.is_empty() {
                    header.trim_end().to_owned()
                } else {
                    kind
                },
            });
        }
        let size: usize = size
            .parse()
            .map_err(|_| IndexError::UnreadableIndexRecord {
                record: header.trim_end().to_owned(),
            })?;
        let mut bytes = vec![0u8; size];
        from_git.read_exact(&mut bytes).map_err(IndexError::Io)?;
        // `--batch` terminates each object with a newline of its own.
        let mut terminator = [0u8; 1];
        from_git
            .read_exact(&mut terminator)
            .map_err(IndexError::Io)?;
        blobs.insert(path.clone(), bytes);
    }
    drop(to_git);
    let _ = child.wait();
    Ok(blobs)
}

/// One path the manifest lists that the index does not agree with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The index holds different bytes from the ones the digest was taken of.
    Stale {
        /// The path.
        path: String,
        /// What the manifest says.
        recorded: String,
        /// What the index's bytes hash to.
        actual: String,
    },
    /// The manifest lists a path the index does not hold.
    NotInTheIndex {
        /// The path.
        path: String,
    },
}

impl Finding {
    /// The path this finding is about.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Self::Stale { path, .. } | Self::NotInTheIndex { path } => path,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale {
                path,
                recorded,
                actual,
            } => write!(f, "{path}: recorded {recorded}, the index holds {actual}"),
            Self::NotInTheIndex { path } => {
                write!(f, "{path}: listed, but the index does not hold it")
            }
        }
    }
}

/// Everything the manifest and the index disagree about.
///
/// Every entry is checked, and every disagreement is returned rather than the
/// first, so one run says the whole state of the file.
#[must_use]
pub fn findings(manifest: &Manifest, index: &Index) -> Vec<Finding> {
    let mut found = Vec::new();
    for entry in &manifest.entries {
        match index.blob(&entry.path) {
            None => found.push(Finding::NotInTheIndex {
                path: entry.path.clone(),
            }),
            Some(bytes) => {
                let actual = digest_of(bytes);
                if actual != entry.digest {
                    found.push(Finding::Stale {
                        path: entry.path.clone(),
                        recorded: entry.digest.clone(),
                        actual,
                    });
                }
            }
        }
    }
    found
}

/// The manifest with every digest taken from the index.
///
/// Order and paths are the manifest's own: this rewrites digests and decides
/// nothing about *which* paths belong in a curated selection, which is a
/// judgement no program should make silently. A path the index does not hold
/// keeps the digest it has, so [`findings`] still reports it.
#[must_use]
pub fn corrected(manifest: &Manifest, index: &Index) -> Manifest {
    let entries = manifest
        .entries
        .iter()
        .map(|entry| match index.blob(&entry.path) {
            None => entry.clone(),
            Some(bytes) => Entry {
                digest: digest_of(bytes),
                path: entry.path.clone(),
            },
        })
        .collect();
    Manifest {
        entries,
        trailing_newline: manifest.trailing_newline,
    }
}

/// Why the manifest could not be checked at all.
#[derive(Debug)]
pub enum ManifestError {
    /// The file could not be read.
    Read {
        /// The path it was read from.
        path: std::path::PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The file is not a manifest.
    Parse(ParseError),
    /// The index could not be read.
    Index(IndexError),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Self::Parse(error) => write!(f, "{MANIFEST} is not a manifest: {error}"),
            Self::Index(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ManifestError {}

/// Read `<root>/SHA256SUMS.txt`.
pub fn read(root: &Path) -> Result<Manifest, ManifestError> {
    let path = root.join(MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|source| ManifestError::Read {
        path: path.clone(),
        source,
    })?;
    parse(&text).map_err(ManifestError::Parse)
}

/// Read the manifest and check it against the index at `root`.
pub fn verify(root: &Path) -> Result<Vec<Finding>, ManifestError> {
    let manifest = read(root)?;
    let paths: Vec<String> = manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let index = Index::read(root, &paths).map_err(ManifestError::Index)?;
    Ok(findings(&manifest, &index))
}

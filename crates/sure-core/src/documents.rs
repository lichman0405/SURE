//! The commands a project's own documentation tells you to run, held as
//! documentation.
//!
//! `docs/architecture/EXECUTION_SAFETY.md` states the rule this module exists to
//! obey, in three lines under a heading of its own:
//!
//! > ## README
//! >
//! > README commands are untrusted documentation. Do not execute arbitrary
//! > README shell text automatically.
//!
//! The task is *"README/spec command and claim extraction"* and the acceptance
//! is that sentence: **extracted commands are untrusted documentation and never
//! auto-executed merely because documented.**
//!
//! # There is one list here, and that is a decision rather than an omission
//!
//! A README makes many statements — that a command works, that a file exists,
//! that the project is finished. **One of them is marked up, and SURE reads only
//! that one.** A command in a document is a fenced code block with a language on
//! it; prose is prose. A rule that pulled requirements out of a sentence would
//! be SURE guessing at the author's meaning and then recording the guess in the
//! author's voice, which is the failure this product exists to catch rather than
//! to commit (`docs/security/THREAT_MODEL.md` T17, requirement hallucination,
//! and `MASTER_PROMPT.md` §3 — *"Inference is not a user requirement."*).
//!
//! So a [`DocumentedCommand`] **is** the claim. The document says *this is how
//! you do it*, and SURE records that the document said so. Whether the claim is
//! true is `P4-T005`'s question: nothing here runs, resolves, installs or checks
//! anything.
//!
//! # What makes "never auto-executed" a property rather than a promise
//!
//! Three things, and none of them is a comment asking a later reader to be
//! careful.
//!
//! **A documented command is not an argument vector.** [`DocumentedCommand`]
//! holds one `String` — the line, as the document wrote it — and has no
//! `program`, no `args`, and no `Vec<String>` of anything. Handing a command to
//! the operating system means splitting shell text into a program and its
//! arguments, and nothing in this crate does that: the only `Command::new` in
//! `sure-core`'s shipped code is `fingerprint/git`'s, which runs a named program
//! with a fixed argument vector. `npm install && npm test` is recorded as one
//! string, because splitting it is where a shell would have to be involved and
//! SURE does not have one.
//!
//! **The provenance travels inside the value, and is derived rather than
//! restated.** [`DocumentedCommand::source`] answers
//! [`IntentSource::ProjectSpec`] and [`DocumentedCommand::authority`] answers
//! [`RequirementAuthority::DocumentedInstruction`] — *"Documentation says it.
//! SURE may check whether the documentation is true."* That sentence is the
//! licence for `P4-T005` and the boundary for this module, and the answer comes
//! from [`Config::goal_source`] rather than from a constant written here, so
//! there is still exactly one place in the crate that decides a file inside the
//! project is documentation.
//!
//! **The report is a result, not an assembly.** [`DocumentReport::of`] is the
//! only door, so no caller can produce one whose commands came from somewhere
//! else or whose [`DocumentReport::is_complete`] disagrees with the files it
//! actually opened.
//!
//! What is deliberately **not** claimed is that any of that is a wall. There is
//! no function anywhere that turns a [`DocumentedCommand`] into an
//! [`ApprovedCommand`](sure_domain::execution::ApprovedCommand), and `sure-cli`
//! builds no [`DocumentReport`] at all — so today the path does not exist rather
//! than being blocked. The day somebody writes it, that commit is the decision,
//! and this paragraph is what says it was one.
//!
//! # What counts as a command
//!
//! A line is recorded when all four of these hold, and each has a reason:
//!
//! 1. **It is inside a fenced code block whose language names a shell.** The
//!    fence is the markup and the tag is the author saying *this is a shell*. A
//!    `json` block holds a document, not a command, and a `python` block holds
//!    code that belongs to the project rather than instructions to a reader.
//! 2. **It is not blank.**
//! 3. **Its first non-blank character is not `#`.** In every language on
//!    [`ShellLanguage`]'s list that starts a comment, so a line beginning with
//!    one is the author's prose set inside the block.
//! 4. **In a `console` or `shell-session` block, it starts with `$ `.** Those
//!    two tags exist to hold a session — the commands and the output they
//!    produced — and the `$` is the convention that tells the two apart. Without
//!    this rule SURE would report `PASS src/app.test.js` as something the README
//!    told you to run.
//!
//! The recorded text is the line with the `$ ` marker removed, and
//! [`DocumentedCommand::prompt`] records that a marker was there, so the
//! transformation is visible in the value rather than hidden by it. Nothing else
//! is changed: SURE does not split a command, expand a variable, resolve a `cd`,
//! or join a line continued with a backslash.
//!
//! **A fenced block with no language on it is not read, and is reported as
//! unread.** Plenty of READMEs put a command in an untagged fence, and from the
//! markup alone SURE cannot tell whether the block holds a shell command or a
//! JSON blob. It says it did not look instead of guessing, and
//! [`DocumentReport::is_complete`] turns false. A report that listed three
//! commands and stayed silent about the five blocks it could not classify would
//! be claiming to have found the commands in a document it did not finish
//! reading.
//!
//! **A tag SURE does not recognise is not a shell, and is not a gap.** `text`,
//! `txt` and `plain` are the common near-misses, and an author who wanted a
//! command read as a command had a shell tag available. Reading a command out of
//! a block the author labelled as plain text would be SURE overriding the very
//! markup it is relying on. Unlike an untagged block this costs no coverage —
//! the author stated something — so it does not make
//! [`DocumentReport::is_complete`] false.
//!
//! # What it does not do
//!
//! **It does not run anything.** That is the whole point; see above.
//!
//! **It does not decide whether a command is dangerous.** `rm -rf /` in a
//! README is recorded in the same shape as `npm test`, because the thing worth
//! reporting is that a project's documentation says to run it. Nothing here
//! reads [`crate::redact`] or any risk table, and a report that ranked these
//! would be inviting a caller to run the ones it ranked low.
//!
//! **It does not parse Markdown.** It finds fences, fence tags and ATX headings
//! and nothing else: no setext headings, no lists, no tables, no link following,
//! no `<!-- -->` handling, and no indented code blocks. A setext heading leaves
//! the previous ATX heading in force, and an indented code block is not read at
//! all — both stated here as limits rather than discovered later.
//!
//! **It ignores indentation when it looks for a fence**, which is the one place
//! it knowingly departs from CommonMark. The reason is the one thing it must not
//! do: a fence inside a list item is indented by the width of the list marker,
//! and four spaces is what people write, so following the specification would
//! report *no commands* for a README whose setup steps are a numbered list, and
//! report the reading as complete. See [`opening_fence`] for the trade in full.
//!
//! **It does not check the claims.** Whether a documented command works, or
//! names a script the manifest declares, is `P4-T005`. [`Self::as_requirements`]
//! is where this module hands them on with their label attached.

use std::fs;
use std::path::{Path, PathBuf};

use sure_domain::intent::{IntentSource, Requirement, RequirementAuthority};

use crate::config::Config;
use crate::discover::Discovery;
use crate::scan::display_path;

/// One line a document in the project gives as a command to run.
///
/// [`Self::text`] is the whole line and the only string here. There is no
/// `program` field, no `args` field and no accessor that returns either — see
/// the module docs for why that is the shape rather than a promise.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentedCommand {
    /// The command, as the document wrote it, with a `$ ` prompt marker
    /// removed if one was there and nothing else changed.
    pub text: String,
    /// The shell family the block said this command was for.
    pub language: ShellLanguage,
    /// Whether the line carried a `$ ` marker that was removed.
    pub prompt: bool,
    /// The nearest heading above it, as the document wrote it.
    ///
    /// The heading is the only thing that says what a command is *for*, and it
    /// is markup rather than prose. It is recorded and never interpreted: SURE
    /// does not read `## Testing` as *this runs the tests*.
    pub section: Option<String>,
    /// The document, relative to the project root.
    pub path: PathBuf,
    /// The 1-based line the command is on.
    pub line: usize,
}

impl DocumentedCommand {
    /// The document as text, with `/` on every platform.
    #[must_use]
    pub fn display_path(&self) -> String {
        display_path(&self.path)
    }

    /// The trust label this command carries.
    ///
    /// Derived from [`Config::goal_source`] rather than restated here, so that
    /// the day a project file is treated as anything other than documentation
    /// the change lands in one place instead of two.
    #[must_use]
    pub const fn source(&self) -> IntentSource {
        Config::goal_source()
    }

    /// How much authority this command carries.
    ///
    /// Always [`RequirementAuthority::DocumentedInstruction`]: a document said
    /// it, so SURE may check whether the document is true and may not treat it
    /// as something the user asked for.
    #[must_use]
    pub const fn authority(&self) -> RequirementAuthority {
        RequirementAuthority::from_source(self.source())
    }

    /// This command as the statement it is, with its provenance attached.
    ///
    /// The bridge to `P2-T011`'s intent model: what a document says arrives
    /// there as a [`Requirement`] whose source is already
    /// [`IntentSource::ProjectSpec`], so the model does not have to be told a
    /// second time where it came from.
    #[must_use]
    pub fn as_requirement(&self) -> Requirement {
        Requirement::new(
            format!("doc:{}:{}", self.display_path(), self.line),
            self.text.clone(),
            self.source(),
        )
        .with_raw_retained(true)
    }
}

/// The shell families a fence tag can name.
///
/// Grouped by family rather than kept as the exact tag, for the reason
/// [`crate::references::ReadForm`] groups its spellings: what a reader needs to
/// know is which shell would run this, not whether the author typed `bash` or
/// `sh`. The tag decides which family a command lands in and nothing else about
/// it is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShellLanguage {
    /// A POSIX shell: `bash`, `sh`, `shell`, `zsh`, `ksh`, `dash`, `fish`,
    /// `ash`.
    Posix,
    /// A recorded session — `console`, `shell-session` — where the `$` marks
    /// the commands and everything else is their output.
    Console,
    /// A Windows command interpreter: `cmd`, `batch`, `bat`, `dos`.
    Windows,
    /// PowerShell: `powershell`, `ps1`, `pwsh`.
    PowerShell,
}

impl ShellLanguage {
    /// Every family, in a fixed order.
    pub const ALL: &'static [Self] = &[Self::Posix, Self::Console, Self::Windows, Self::PowerShell];

    /// The family this tag names, or `None` if it names no shell.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let tag = tag.to_ascii_lowercase();
        if POSIX_TAGS.contains(&tag.as_str()) {
            return Some(Self::Posix);
        }
        if CONSOLE_TAGS.contains(&tag.as_str()) {
            return Some(Self::Console);
        }
        if WINDOWS_TAGS.contains(&tag.as_str()) {
            return Some(Self::Windows);
        }
        if POWERSHELL_TAGS.contains(&tag.as_str()) {
            return Some(Self::PowerShell);
        }
        None
    }

    /// A short stable name for this family.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Posix => "posix",
            Self::Console => "console",
            Self::Windows => "windows",
            Self::PowerShell => "powershell",
        }
    }

    /// What this family is, as a fragment of a sentence.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Posix => "a POSIX shell",
            Self::Console => "a recorded shell session",
            Self::Windows => "the Windows command interpreter",
            Self::PowerShell => "PowerShell",
        }
    }

    /// Whether only `$`-marked lines in a block of this family are commands.
    ///
    /// True for [`Self::Console`] alone: the two tags in that family exist
    /// precisely to hold a session, in which everything not marked with a `$`
    /// is the output of the line above it.
    #[must_use]
    pub const fn needs_a_prompt(self) -> bool {
        matches!(self, Self::Console)
    }
}

/// Tags that name a POSIX shell.
const POSIX_TAGS: &[&str] = &["bash", "sh", "shell", "zsh", "ksh", "dash", "fish", "ash"];

/// Tags that name a recorded session rather than a plain command block.
const CONSOLE_TAGS: &[&str] = &["console", "shell-session"];

/// Tags that name the Windows command interpreter.
const WINDOWS_TAGS: &[&str] = &["cmd", "batch", "bat", "dos"];

/// Tags that name PowerShell.
const POWERSHELL_TAGS: &[&str] = &["powershell", "ps1", "pwsh"];

/// A file this pass meant to read and did not, or a block it could not classify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unread {
    /// The document, relative to the project root.
    pub path: PathBuf,
    /// Why it was not read.
    pub reason: UnreadReason,
}

impl Unread {
    /// The document as text, with `/` on every platform.
    #[must_use]
    pub fn display_path(&self) -> String {
        display_path(&self.path)
    }
}

/// Why a document, or a block inside one, was not read.
///
/// Every arm here loses coverage, so every arm makes
/// [`DocumentReport::is_complete`] false. There is no arm for a block whose tag
/// SURE recognised and that is not shell — a `json` block is not *unread*, it is
/// decided, and listing it here would report a decision as a loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnreadReason {
    /// A fenced code block with no language on it.
    ///
    /// The block might hold a command, and the markup does not say. SURE reports
    /// that it did not look.
    UntaggedFence {
        /// The 1-based line the block opens on.
        line: usize,
    },
    /// The pass had already read as many documents as it will in one run.
    OutOfBudget {
        /// How many files one pass reads.
        limit: usize,
    },
    /// The pass had already read as many bytes as it will in one run.
    OutOfBytes {
        /// The limit on the whole pass.
        limit: u64,
    },
    /// Bigger than [`DocumentOptions::max_file_bytes`].
    ///
    /// Checked against the file's metadata before it is opened, so the number
    /// here is the limit that was reached rather than a measurement of the file:
    /// a file one byte over and a file a gigabyte over are refused at the same
    /// point, and SURE has read neither.
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
    /// Decoding lossily would be worse than reporting the file here: a
    /// replacement character could land inside a command and change what a
    /// report says the document asked for, silently.
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
            Self::UntaggedFence { .. } => {
                "a code block had no language on it, so SURE could not tell a command \
                 from anything else in it"
            }
            Self::OutOfBudget { .. } => {
                "SURE had already read as many documents as it reads in one pass"
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
/// Smaller than [`crate::references::ReferenceOptions`]'s because the files are:
/// a document is prose with examples in it, and a project with more than a few
/// hundred of them is one whose documentation SURE should sample rather than
/// consume. These are this pass's numbers rather than a second copy of that
/// pass's, and the two are free to move apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentOptions {
    /// The largest document this pass will open. Default 1 MiB.
    pub max_file_bytes: u64,
    /// How many documents one pass will read. Default 256.
    pub max_files: usize,
    /// How many bytes one pass will read in total. Default 8 MiB.
    pub max_total_bytes: u64,
}

impl Default for DocumentOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 1024 * 1024,
            max_files: 256,
            max_total_bytes: 8 * 1024 * 1024,
        }
    }
}

impl DocumentOptions {
    /// The same options with a different per-file size limit.
    #[must_use]
    pub const fn with_max_file_bytes(mut self, max_file_bytes: u64) -> Self {
        self.max_file_bytes = max_file_bytes;
        self
    }

    /// The same options with a different document count.
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

/// Whether a path is a document this pass reads.
///
/// A `README*` file at any extension, or anything ending in `.md`. The rule
/// lives here because this is the module about documents, and
/// [`crate::references::declaration_candidate`] asks this rather than testing
/// the two name shapes again — two copies of a naming rule is how the two
/// passes would come to disagree about which files are documents.
///
/// Case-insensitive, because Windows and macOS are.
#[must_use]
pub fn is_document(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    let name = name.to_string_lossy().to_lowercase();
    name.starts_with("readme") || name.ends_with(".md")
}

/// Every command a project's documents give, and what SURE did not read.
///
/// Built by [`Self::of`], which is the only door: the report is a result rather
/// than a value a caller assembles, so there is no way to produce one whose
/// [`Self::is_complete`] does not match the files it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentReport {
    /// The project root the paths are relative to.
    pub root: PathBuf,
    /// Every document this pass opened, in path order.
    ///
    /// Here because *no commands* and *no documents* are different facts, and a
    /// report that showed only the first would let a project with no README look
    /// like a README with no commands in it.
    pub documents: Vec<PathBuf>,
    /// Every command found, in document-and-line order.
    pub commands: Vec<DocumentedCommand>,
    /// Every document or block this pass meant to read and did not.
    pub unread: Vec<Unread>,
    /// The limits this pass ran under.
    pub options: DocumentOptions,
    /// Whether the scan underneath this pass walked everything it set out to.
    ///
    /// Private because it is only meaningful through [`Self::is_complete`]: a
    /// directory the walk could not enter holds documents, and a caller that
    /// read this field and nothing else would be asking half the question.
    scan_was_complete: bool,
}

impl DocumentReport {
    /// Read a discovery's documents, under the defaults.
    #[must_use]
    pub fn of(discovery: &Discovery) -> Self {
        Self::with_options(discovery, &DocumentOptions::default())
    }

    /// Read a discovery's documents.
    ///
    /// The scan in `discovery` decides *which* files are documents; this pass
    /// opens them. Nothing is executed, nothing is resolved, and the only files
    /// opened are the ones [`is_document`] accepts.
    #[must_use]
    pub fn with_options(discovery: &Discovery, options: &DocumentOptions) -> Self {
        let mut documents = Vec::new();
        let mut commands = Vec::new();
        let mut unread = Vec::new();

        let mut reader = Reader::new(options);
        for path in candidates(&discovery.scan) {
            let text = match reader.read(discovery, &path) {
                Ok(text) => text,
                Err(reason) => {
                    unread.push(Unread { path, reason });
                    continue;
                }
            };
            let (found, missed) = commands_in(&text, &path);
            documents.push(path);
            commands.extend(found);
            unread.extend(missed);
        }

        // By document and line rather than by the derived order, which would put
        // every POSIX command in the project before every PowerShell one. A
        // person reading this list wants to walk the documents.
        commands.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));

        Self {
            root: discovery.root.clone(),
            documents,
            commands,
            unread,
            options: *options,
            scan_was_complete: discovery.scan.is_complete(),
        }
    }

    /// Every command found, in document-and-line order.
    #[must_use]
    pub fn commands(&self) -> &[DocumentedCommand] {
        &self.commands
    }

    /// Every document this pass opened, in path order.
    #[must_use]
    pub fn documents(&self) -> &[PathBuf] {
        &self.documents
    }

    /// Every document or block this pass meant to read and did not.
    #[must_use]
    pub fn unread(&self) -> &[Unread] {
        &self.unread
    }

    /// Whether everything this pass set out to read was read.
    ///
    /// **A caller that reports a missing command without asking this first is
    /// making a claim about a project SURE did not finish reading.** False when
    /// any document went unread for any reason, false when any code block had no
    /// language on it, and false when the scan underneath could not walk some
    /// part of the project.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unread.is_empty() && self.scan_was_complete
    }

    /// The commands as the statements they are, each with its provenance.
    ///
    /// The bridge to `P2-T011`. Every one arrives as a [`Requirement`] whose
    /// source is documentation, so nothing downstream has to be told where it
    /// came from and [`sure_domain::intent::may_claim_full_fulfilment`] cannot
    /// count one of these towards what the user asked for.
    #[must_use]
    pub fn as_requirements(&self) -> Vec<Requirement> {
        self.commands
            .iter()
            .map(DocumentedCommand::as_requirement)
            .collect()
    }

    /// What this pass found, in one sentence.
    ///
    /// The counts are data and the sentence is a constant, and the sentence is
    /// worded so that it is true of a partial reading: it says what SURE *read*,
    /// never what the project has.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let documents = self.documents.len();
        let commands = self.commands.len();
        let mut sentence = format!(
            "SURE read {documents} {} and found {commands} {}",
            if documents == 1 {
                "document"
            } else {
                "documents"
            },
            if commands == 1 { "command" } else { "commands" }
        );
        if !self.is_complete() {
            sentence.push_str(
                ", and did not finish reading every code block in them, so a command it did \
                 not find may be one it did not look for",
            );
        }
        sentence.push('.');
        sentence
    }

    /// What SURE did not do with these commands.
    ///
    /// A constant rather than a conditional, because there is no state of this
    /// report in which a documented command was run or is about to be. It is the
    /// acceptance condition written for a reader rather than for a test, and it
    /// sits next to [`Self::plain_description`] so that a report which prints
    /// one can print the other.
    #[must_use]
    pub const fn execution_note() -> &'static str {
        "SURE has not run any of these commands, and will not run one just because a \
         document in the project says to."
    }
}

/// Every document in a scan, in a fixed order.
///
/// Sorted by path so that two runs over the same project read the files in the
/// same order. The budget makes that matter: which document is the one that runs
/// out of budget is a result, and it must not depend on the order a directory
/// happened to come back in.
fn candidates(scan: &crate::scan::Scan) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = scan
        .files()
        .filter(|entry| is_document(&entry.path))
        .map(|entry| entry.path.clone())
        .collect();
    found.sort();
    found
}

/// Reads candidate documents, spending a budget and saying what it did not read.
struct Reader<'a> {
    options: &'a DocumentOptions,
    files_read: usize,
    bytes_read: u64,
}

impl Reader<'_> {
    fn new(options: &DocumentOptions) -> Reader<'_> {
        Reader {
            options,
            files_read: 0,
            bytes_read: 0,
        }
    }

    fn read(&mut self, discovery: &Discovery, path: &Path) -> Result<String, UnreadReason> {
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

        let full = discovery.root.join(path);
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

        match fs::read(&full) {
            Ok(bytes) => {
                self.files_read += 1;
                self.bytes_read += bytes.len() as u64;
                String::from_utf8(bytes).map_err(|error| UnreadReason::NotText {
                    detail: error.to_string(),
                })
            }
            Err(error) => Err(UnreadReason::Unreadable {
                detail: error.to_string(),
            }),
        }
    }
}

/// Whether a fence's language means SURE reads the block, and what was on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FenceLanguage {
    /// A shell SURE knows.
    Shell(ShellLanguage),
    /// A language SURE read the tag for and that is not a shell.
    Other,
    /// No language on the fence at all.
    Unstated,
}

/// A fenced code block that is open.
#[derive(Debug, Clone, Copy)]
struct OpenFence {
    marker: u8,
    length: usize,
    language: FenceLanguage,
}

/// Every command in one document, and every block SURE could not classify.
fn commands_in(text: &str, path: &Path) -> (Vec<DocumentedCommand>, Vec<Unread>) {
    let mut commands = Vec::new();
    let mut unread = Vec::new();
    let mut open: Option<OpenFence> = None;
    let mut section: Option<String> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        match open {
            Some(fence) => {
                if closes(raw, &fence) {
                    open = None;
                } else if let FenceLanguage::Shell(language) = fence.language
                    && let Some(command) = command_from(raw, language, section.clone(), path, line)
                {
                    commands.push(command);
                }
            }
            None => {
                if let Some(fence) = opening_fence(raw) {
                    if fence.language == FenceLanguage::Unstated {
                        unread.push(Unread {
                            path: path.to_path_buf(),
                            reason: UnreadReason::UntaggedFence { line },
                        });
                    }
                    open = Some(fence);
                } else if let Some(heading) = heading_of(raw) {
                    section = Some(heading);
                }
            }
        }
    }

    (commands, unread)
}

/// The one line a document gives as a command, or `None` if it gives none.
fn command_from(
    raw: &str,
    language: ShellLanguage,
    section: Option<String>,
    path: &Path,
    line: usize,
) -> Option<DocumentedCommand> {
    let trimmed = raw.trim();
    // Blank lines and comments are the author's prose set inside the block, not
    // commands. `#` opens a comment in every shell on the list.
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // In a recorded session everything that is not marked with a `$` is the
    // output of the line above it.
    //
    // There is no check that `text` is non-empty, and it is worth saying why
    // rather than leaving a reader to add one. `trim` has already taken any
    // trailing whitespace off, so a line that is only `$` no longer matches
    // `"$ "` and the `?` above returns `None`; every line that does match keeps
    // a non-whitespace character after the space, because otherwise `trim` would
    // have removed it. A guard here would be unreachable, and an unreachable
    // guard reads as a safety net that is not there.
    let (text, prompt) = if language.needs_a_prompt() {
        let rest = trimmed.strip_prefix("$ ")?.trim_start();
        (rest, true)
    } else {
        (trimmed, false)
    };

    Some(DocumentedCommand {
        text: text.to_owned(),
        language,
        prompt,
        section,
        path: path.to_path_buf(),
        line,
    })
}

/// The language a fence's info string names, with a pandoc attribute wrapper
/// removed, so that ```` ```{.bash} ```` and ```` ```bash title="x" ```` both
/// name `bash`.
fn tag_of(info: &str) -> Option<&str> {
    let word = info.split_whitespace().next()?;
    let word = word.trim_start_matches('{').trim_start_matches('.');
    let word = word.trim_end_matches('}');
    if word.is_empty() { None } else { Some(word) }
}

/// The fence a line opens, if it opens one.
///
/// **Indentation is ignored, which is a deliberate departure from CommonMark.**
/// The specification allows a fence up to three spaces in, and calls four or
/// more an indented code block. But a fence inside a list item is indented by
/// the width of the list marker, and four spaces is what people write by hand —
/// so following the rule exactly would walk past
///
/// ```text
/// -   Install:
///     ```bash
///     npm install
///     ```
/// ```
///
/// and report a document with no commands in it and a complete reading, which
/// is the false green this repository treats as worse than a visible error. The
/// cost of ignoring indentation is the other direction: a fence shown literally
/// inside an indented code block is read as a real one. That is a finding a
/// person can go and look at in the document, and the text really is there,
/// which is the trade `CLAUDE.md`'s ordering picks.
fn opening_fence(line: &str) -> Option<OpenFence> {
    let rest = line.trim_start();
    let marker = *rest.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let length = rest.bytes().take_while(|byte| *byte == marker).count();
    if length < 3 {
        return None;
    }
    let info = rest[length..].trim();
    // A backtick fence's info string may not contain a backtick, which is what
    // keeps `` `code` `` from opening a block that would never close.
    if marker == b'`' && info.contains('`') {
        return None;
    }
    let language = match tag_of(info) {
        Some(tag) => match ShellLanguage::from_tag(tag) {
            Some(shell) => FenceLanguage::Shell(shell),
            None => FenceLanguage::Other,
        },
        None => FenceLanguage::Unstated,
    };
    Some(OpenFence {
        marker,
        length,
        language,
    })
}

/// Whether a line closes the open fence.
///
/// The marker must be the same character and at least as long, and nothing may
/// follow it — a line that looks like a fence but carries an info string of its
/// own is content, not a closer.
fn closes(line: &str, fence: &OpenFence) -> bool {
    let rest = line.trim_start();
    let run = rest
        .bytes()
        .take_while(|byte| *byte == fence.marker)
        .count();
    run >= fence.length && rest[run..].trim().is_empty()
}

/// An ATX heading's text, if the line is one.
///
/// Setext headings — a line underlined with `=` or `-` — are deliberately not
/// recognised. The module docs say so; the cost is that a document written
/// entirely in setext headings has no sections on its commands.
fn heading_of(line: &str) -> Option<String> {
    let rest = unindent(line);
    let hashes = rest.bytes().take_while(|byte| *byte == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let after = &rest[hashes..];
    if !(after.is_empty() || after.starts_with(' ')) {
        return None;
    }
    let text = after.trim().trim_end_matches('#').trim_end();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

/// A line with up to three leading spaces removed, which is as far as CommonMark
/// allows a **heading** to be indented.
///
/// Fences do not use this — see [`opening_fence`] — and the asymmetry is
/// deliberate. A fence is somewhere a command might be, so missing one loses a
/// finding; a heading is only a label on a finding that is reported either way,
/// so it is worth being strict about and not worth deviating from the
/// specification for.
fn unindent(line: &str) -> &str {
    let mut spaces = 0;
    for byte in line.bytes() {
        if byte == b' ' && spaces < 3 {
            spaces += 1;
        } else {
            break;
        }
    }
    &line[spaces..]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Every command in one document, with nothing else in the way.
    fn commands(text: &str) -> Vec<DocumentedCommand> {
        commands_in(text, Path::new("README.md")).0
    }

    /// The text of every command in one document.
    fn texts(text: &str) -> Vec<String> {
        commands(text)
            .into_iter()
            .map(|command| command.text)
            .collect()
    }

    #[test]
    fn a_shell_fence_yields_its_lines_as_commands() {
        assert_eq!(
            texts("```bash\nnpm install\nnpm test\n```\n"),
            ["npm install", "npm test"]
        );
    }

    #[test]
    fn every_shell_family_is_read() {
        for tag in ["bash", "sh", "zsh", "fish", "cmd", "powershell", "pwsh"] {
            let document = format!("```{tag}\ndeploy --now\n```\n");
            assert_eq!(texts(&document), ["deploy --now"], "tag {tag:?}");
        }
    }

    #[test]
    fn the_tag_is_matched_case_insensitively_and_may_carry_extra_words() {
        for tag in [
            "BASH",
            "Bash",
            "bash title=\"setup\"",
            "{.bash}",
            "{.bash .numberLines}",
        ] {
            let document = format!("```{tag}\nnpm ci\n```\n");
            assert_eq!(texts(&document), ["npm ci"], "tag {tag:?}");
        }
    }

    #[test]
    fn a_fence_in_another_language_yields_nothing() {
        // The author said what this block is, and it is not a shell. Reading a
        // command out of it would be SURE overriding the markup it relies on.
        for tag in [
            "json", "yaml", "python", "js", "rust", "text", "txt", "plain",
        ] {
            let document = format!("```{tag}\nnpm install\n```\n");
            assert!(texts(&document).is_empty(), "tag {tag:?}");
        }
    }

    #[test]
    fn a_tag_that_is_not_a_shell_is_a_decision_and_not_a_gap() {
        // The difference from an untagged block: the author stated something, so
        // there is no coverage to lose and the reading is still complete.
        let (_, unread) = commands_in("```json\n{}\n```\n", Path::new("README.md"));
        assert!(unread.is_empty());
    }

    #[test]
    fn an_untagged_fence_yields_nothing_and_is_reported_with_its_line() {
        let (found, unread) =
            commands_in("# Title\n\n```\nnpm install\n```\n", Path::new("README.md"));
        assert!(found.is_empty(), "an untagged block is not read");
        assert_eq!(
            unread,
            [Unread {
                path: PathBuf::from("README.md"),
                reason: UnreadReason::UntaggedFence { line: 3 },
            }]
        );
    }

    #[test]
    fn a_comment_inside_a_shell_fence_is_not_a_command() {
        assert_eq!(
            texts("```bash\n# Install the dependencies\nnpm install\n```\n"),
            ["npm install"]
        );
    }

    #[test]
    fn a_blank_line_inside_a_fence_is_not_a_command() {
        assert_eq!(texts("```bash\n\n   \nnpm test\n\n```\n"), ["npm test"]);
    }

    #[test]
    fn a_console_block_records_only_what_follows_a_prompt() {
        // The two console tags exist to hold a session. Without this rule the
        // output lines would be reported as commands the README gave.
        assert_eq!(
            texts("```console\n$ npm test\n\n> jest\n\nPASS src/app.test.js\n```\n"),
            ["npm test"]
        );
    }

    #[test]
    fn the_prompt_marker_is_removed_and_recorded() {
        let found = commands("```console\n$ npm run build\n```\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "npm run build");
        assert!(
            found[0].prompt,
            "the marker was there and the value says so"
        );
        assert_eq!(found[0].language, ShellLanguage::Console);
    }

    #[test]
    fn a_console_block_with_no_prompt_anywhere_yields_nothing() {
        let (found, unread) = commands_in("```console\nnpm test\n```\n", Path::new("README.md"));
        assert!(found.is_empty());
        // Still not a gap: the tag was read, and it is a shell SURE knows.
        assert!(unread.is_empty());
    }

    #[test]
    fn a_prompt_in_a_plain_shell_fence_is_left_alone() {
        // Only the console family says a `$` means "this is the input". In a
        // `bash` block the author did not say that, so the `$` is part of what
        // the document wrote.
        let found = commands("```bash\n$ npm test\n```\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "$ npm test");
        assert!(!found[0].prompt);
    }

    #[test]
    fn a_dollar_sign_that_is_not_a_prompt_does_not_make_a_command() {
        // The marker is `$` and a space. A line starting `$HOME/...` is a
        // command whose first word is a variable, and in a console block it is
        // output — reading it as input would report a command that began at the
        // wrong character.
        let (found, unread) = commands_in(
            "```console\n$HOME/bin/deploy\n```\n",
            Path::new("README.md"),
        );
        assert!(found.is_empty());
        assert!(unread.is_empty());
    }

    #[test]
    fn a_prompt_with_nothing_after_it_is_not_a_command() {
        // A `$` alone is the author showing an empty prompt, and recording it
        // would put a command with no text in the report.
        //
        // Nothing rejects these two lines by hand. `trim` takes the trailing
        // whitespace off first, so `strip_prefix("$ ")` does not match and `?`
        // returns `None` — which is what makes the space in `"$ "` load-bearing
        // rather than cosmetic, and is why `command_from` has no empty-text
        // check. This test is where that invariant lives now: weaken the `trim`
        // to a `trim_start` and it is this test, not a guard, that fails.
        let (found, unread) = commands_in(
            "```console\n$\n$   \nnpm test\n```\n",
            Path::new("README.md"),
        );
        assert!(found.is_empty());
        assert!(unread.is_empty());
    }

    #[test]
    fn the_text_is_recorded_exactly_as_the_document_wrote_it() {
        let found = commands("```bash\n  npm run build -- --watch  \n```\n");
        assert_eq!(found[0].text, "npm run build -- --watch");
    }

    #[test]
    fn a_command_keeps_its_shell_operators_unsplit() {
        // The acceptance, as a property of the value: what comes out is one
        // string. Splitting it into a program and arguments is the act that
        // would have to happen before anything could run, and it does not happen
        // here and is not stored here.
        let found = commands("```bash\nnpm install && npm test\n```\n");
        assert_eq!(found.len(), 1, "one line is one command, not two");
        assert_eq!(found[0].text, "npm install && npm test");
        assert_eq!(
            found[0].text.split_whitespace().count(),
            5,
            "the recording is textual; nothing here has parsed it"
        );
    }

    #[test]
    fn a_documented_command_is_documentation_and_not_a_user_requirement() {
        let found = commands("```bash\nnpm test\n```\n");
        let command = &found[0];
        assert_eq!(command.source(), IntentSource::ProjectSpec);
        assert_eq!(
            command.authority(),
            RequirementAuthority::DocumentedInstruction
        );
        assert!(
            !command.source().is_user_requirement(),
            "a README is somebody's documentation, not the user's request"
        );
        assert!(command.source().is_documentation());
    }

    #[test]
    fn the_command_arrives_at_the_intent_model_already_labelled() {
        let command = commands("```bash\nnpm test\n```\n").remove(0);
        let requirement = command.as_requirement();
        assert_eq!(requirement.source, IntentSource::ProjectSpec);
        assert_eq!(
            requirement.authority(),
            RequirementAuthority::DocumentedInstruction
        );
        assert!(!requirement.is_user_requirement());
        assert_eq!(requirement.id, "doc:README.md:2");
        assert_eq!(requirement.text, "npm test");
        assert!(
            requirement.raw_retained,
            "the whole line was kept, so this is the raw wording rather than a summary"
        );
    }

    #[test]
    fn a_command_carries_the_heading_it_sits_under() {
        let found = commands(
            "# Project\n\n## Getting started\n\n```bash\nnpm install\n```\n\n\
             ## Testing\n\n```bash\nnpm test\n```\n",
        );
        assert_eq!(
            found
                .iter()
                .map(|command| command.section.as_deref())
                .collect::<Vec<_>>(),
            [Some("Getting started"), Some("Testing")]
        );
    }

    #[test]
    fn a_heading_inside_a_fence_is_not_a_heading() {
        let found = commands("## Setup\n\n```bash\n# Not a heading\nnpm install\n```\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].section.as_deref(), Some("Setup"));
    }

    #[test]
    fn a_hash_without_a_space_is_not_a_heading() {
        assert!(heading_of("#hashtag").is_none());
        assert!(heading_of("####### seven").is_none());
        assert!(heading_of("#").is_none());
        assert_eq!(heading_of("## Testing ##").as_deref(), Some("Testing"));
        assert_eq!(heading_of("   ### Deep").as_deref(), Some("Deep"));
    }

    #[test]
    fn a_setext_heading_is_not_a_heading() {
        // Stated in the module docs as a limit rather than left to be found.
        assert!(heading_of("Testing\n").is_none());
        assert!(heading_of("=======\n").is_none());
    }

    #[test]
    fn a_fence_ends_at_its_closing_marker() {
        assert_eq!(
            texts("```bash\nnpm install\n```\nnpm test\n"),
            ["npm install"],
            "the line after the closing fence is prose"
        );
    }

    #[test]
    fn a_longer_or_equal_marker_closes_and_a_shorter_one_does_not() {
        assert_eq!(texts("```bash\nnpm ci\n````\nnpm test\n"), ["npm ci"]);
        assert_eq!(texts("```bash\nnpm ci\n``\n"), ["npm ci", "``"]);
    }

    #[test]
    fn a_different_marker_does_not_close_a_fence() {
        // `~~~` inside a backtick fence is content, and it is one line here
        // rather than a second fence that swallows the rest of the file.
        assert_eq!(
            texts("```bash\nnpm ci\n~~~\nnpm test\n```\n"),
            ["npm ci", "~~~", "npm test"]
        );
    }

    #[test]
    fn a_tilde_fence_is_a_fence_too() {
        assert_eq!(texts("~~~bash\nnpm test\n~~~\n"), ["npm test"]);
    }

    #[test]
    fn a_fence_inside_a_list_is_read() {
        // The departure from CommonMark, pinned so it is a decision rather than
        // an accident. Every one of these is indented at least four spaces, and
        // every one of them is a command a person reading the rendered document
        // sees as a command. Reporting zero commands and a complete reading for
        // the first of them is the false green this rule exists to prevent.
        for document in [
            "-   Install:\n    ```bash\n    npm install\n    ```\n",
            "1. Install:\n\n    ```bash\n    npm install\n    ```\n",
            "        ```bash\n        npm install\n        ```\n",
            "  - deep:\n    - deeper:\n      ```bash\n      npm install\n      ```\n",
        ] {
            assert_eq!(texts(document), ["npm install"], "{document:?}");
        }
    }

    #[test]
    fn a_fence_inside_a_list_still_needs_its_language() {
        // Reading past the indentation does not mean reading past the tag: an
        // indented untagged block is still a block SURE could not classify.
        let (found, unread) = commands_in(
            "-   Install:\n    ```\n    npm install\n    ```\n",
            Path::new("README.md"),
        );
        assert!(found.is_empty());
        assert_eq!(
            unread,
            [Unread {
                path: PathBuf::from("README.md"),
                reason: UnreadReason::UntaggedFence { line: 2 },
            }]
        );
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end_of_the_document() {
        assert_eq!(
            texts("# Title\n\n```bash\nnpm a\nnpm b\n"),
            ["npm a", "npm b"]
        );
    }

    #[test]
    fn a_line_carrying_an_info_string_does_not_close_a_fence() {
        // A closer may not carry an info string, so this is content.
        assert_eq!(
            texts("```bash\nnpm ci\n```bash\nnpm test\n```\n"),
            ["npm ci", "```bash", "npm test"]
        );
    }

    #[test]
    fn inline_code_does_not_open_a_fence() {
        // The info string of a backtick fence may not contain a backtick, which
        // is what keeps this line from opening a block that never closes.
        assert_eq!(
            texts("Run `` `npm test` `` to test.\n\n```bash\nnpm run build\n```\n"),
            ["npm run build"]
        );
    }

    #[test]
    fn two_fences_are_two_blocks() {
        let (found, unread) = commands_in(
            "```bash\nnpm install\n```\n\n```\nnot read\n```\n\n```bash\nnpm test\n```\n",
            Path::new("README.md"),
        );
        assert_eq!(
            found.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["npm install", "npm test"]
        );
        assert_eq!(unread.len(), 1, "the untagged block between them");
    }

    #[test]
    fn a_command_records_the_line_it_is_on() {
        let found = commands("line one\n\n```bash\nnpm test\n```\n");
        assert_eq!(found[0].line, 4);
        assert_eq!(found[0].display_path(), "README.md");
    }

    #[test]
    fn a_document_with_no_fence_at_all_yields_nothing_and_no_gap() {
        let (found, unread) = commands_in("# Title\n\nJust prose.\n", Path::new("README.md"));
        assert!(found.is_empty());
        assert!(unread.is_empty());
    }

    #[test]
    fn the_document_predicate_is_case_insensitive_and_names_one_shape() {
        for name in [
            "README.md",
            "readme",
            "ReadMe.rst",
            "docs/setup.md",
            "NOTES.MD",
        ] {
            assert!(is_document(Path::new(name)), "{name:?}");
        }
        for name in [
            "package.json",
            "src/app.js",
            ".env.example",
            "notes.txt",
            // Ends in `md`, and is a Windows batch file rather than a document.
            "build.cmd",
        ] {
            assert!(!is_document(Path::new(name)), "{name:?}");
        }
    }

    #[test]
    fn the_execution_note_says_what_did_not_happen() {
        let note = DocumentReport::execution_note();
        assert!(note.contains("has not run"), "{note}");
        assert!(note.contains("will not run"), "{note}");
    }
}

//! The keys a project asks for and the keys it names, settled into a verdict.
//!
//! [`crate::references`] is the reading half of this check and answers a
//! question about two lists: which keys a project's source files ask the
//! environment for, and which keys its example files and documents name. What it
//! deliberately does not do is draw a conclusion. Its module documentation says
//! so in one sentence — *"Neither is a claim about the project unless the
//! reading was complete"* — and this module is that sentence's other half.
//!
//! # Why one module reads and another settles
//!
//! `docs/architecture/EVIDENCE_MODEL.md` requires an anchor for every material
//! claim, and an anchor is only worth having if the hand that writes it is the
//! hand that knows what was read. A reader that also issued verdicts would have
//! to hold a [`FingerprintId`] and decide severities, and the day somebody
//! wanted a key list for a display that settles nothing they would get the
//! verdicts anyway. [`crate::documents`] and [`crate::setup`] are split for the
//! same reason, and this module follows the shape they settled on: the reading
//! module keeps its own tests, and the assessment module is where a verdict and
//! its evidence are built together so that neither can exist without the other.
//!
//! # The rule that decides every verdict here
//!
//! **A one-sided key is a claim only if the reading finished.** So
//! [`CompletenessReport::of`] asks [`ReferenceReport::is_complete`] once and
//! every claim takes the same answer:
//!
//! - **The reading finished** — the claim is [`ClaimAssessment::Confirmed`],
//!   and it is confirmed *as worded*: that a source file asks for the key and
//!   that no example file or document names it. Both halves are things SURE
//!   read, and [`AssessedKey::reason`] says which file and which line the first
//!   half is at.
//! - **The reading did not finish** — the claim is
//!   [`ClaimAssessment::CannotConfirm`], carries **no evidence at all**, and its
//!   reason names what went unread. An anchor on a claim SURE could not settle
//!   would be evidence for nothing, which is what [`crate::checks::evidence_of`]
//!   refuses one layer up.
//!
//! **Nothing here is ever [`ClaimAssessment::Contradicted`]**, and that is a
//! property of the subject rather than a gap. A contradicted claim is one a
//! project made and SURE refuted — a README naming a file that is not there. An
//! undeclared key refutes nothing: no file in the project says the key is
//! documented, so there is no statement for the finding to be the opposite of.
//! [`crate::setup`] reaches `Contradicted` because a document *does* claim its
//! paths exist; this module has no such claim to work with.
//!
//! # The value rule, arriving at the layer that writes sentences
//!
//! `SECRET_REDACTION.md` and `PRIVACY.md` are the terms here, and
//! [`crate::references`] already answers them structurally: a
//! [`Reference`](crate::references::Reference) has no field a value could be in.
//! This layer writes the sentences a person reads, which is where a value would
//! matter, and it answers the same way twice over:
//!
//! - **A reason sentence contains a key name and a path, and nothing else the
//!   project wrote.** A key name is safe to place for a reason a reader can
//!   check: [`crate::references`]'s extractors accept a name only when the name
//!   is a plain identifier, so every name is `[A-Za-z_][A-Za-z0-9_]*` and no
//!   name can carry a newline, a quote, or anything else that would add a line
//!   to a message. A path is escaped nowhere here because it is placed by
//!   [`display_path`], which renders the components of a path this pass measured
//!   rather than text a project wrote.
//! - **No anchor sets an excerpt.** [`EvidenceAnchor`] offers one field that
//!   exists to hold the text a claim was read from, and the line a key is read
//!   on is exactly where a value lives: `process.env.API_KEY = "…"` is one line
//!   and the reading half is the other half of it. Every anchor this module
//!   builds leaves the excerpt empty, which is a weaker promise than
//!   [`crate::references`]'s — *nothing here quotes a project line at all* — and
//!   a stronger one than redaction can make, because there is no text to redact.
//!   `every_anchor_is_empty_of_excerpts` is what holds it.
//!
//! # What it does not do
//!
//! **It does not read `.env`.** [`crate::references`] opens a `.env`-family file
//! only when its name marks it a template, and this module reads no file of its
//! own — every fact here comes through [`ReferenceReport`]. The file the values
//! are in is not opened, not parsed, and not reported unread.
//!
//! **It does not know what a key is for.** `DATABASE_URL` and `PORT` are two
//! names. SURE has read no schema, no compose file and no deployment manifest,
//! so it cannot say that a key is spelled the way some consumer spells it, nor
//! that a key nothing reads is unused. A key can be read by a shell script, a
//! `Dockerfile`, a CI workflow, a `Makefile` or a language SURE does not read,
//! and every one of those is a way for
//! [`KeyStatus::DeclaredButNotRead`] to be true of a key that is in daily use.
//! The reason sentence for that claim says so rather than leaving the reader to
//! work it out.
//!
//! **It does not treat every undeclared key as something the project owes.**
//! `PATH`, `HOME`, `NODE_ENV` and the rest are set for a process before the
//! project starts, and no project should document them. [`PROVIDED_BY_RUNTIME`]
//! is the list of names SURE will not count against a project; a key on it still
//! gets a claim, so that a reader sees the key and SURE's judgement about it, but
//! the claim carries [`Severity::Note`] and [`CompletenessReport::set_aside`]
//! separates it from the keys the project owes something for. The list is SURE's
//! own reading of what an operating system and a language runtime provide, it is
//! a public constant rather than a rule buried in a function, and matching is
//! exact and case-insensitive — see [`provided_by_runtime`].
//!
//! **It does not look for a key anywhere but the two sides.** A key mentioned in
//! a `Dockerfile` is not a read and is not a declaration, which is
//! [`crate::references`]'s scope arriving here unchanged. It is also the
//! limitation that makes the sentence for a [`KeyStatus::DeclaredButNotRead`]
//! claim end in *"so this is what SURE read rather than a statement that the key
//! is unused"* instead of in a finding.

use std::path::{Path, PathBuf};

use sure_domain::evidence::{
    AnchorSubject, ClaimAssessment, Evidence, EvidenceAnchor, EvidenceClass,
};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

use crate::discover::Discovery;
use crate::references::{KeyReport, KeyStatus, ReferenceOptions, ReferenceReport, Unread};
use crate::scan::display_path;

/// How many places one claim anchors.
///
/// A key can be read on every line of a generated file, and an evidence list
/// that grew with the number of reads would be a list with no bound on it. Eight
/// is more than a reader follows and fewer than a project can produce, and
/// [`AssessedKey::reason`] says how many places there were whenever the list was
/// cut.
pub const MAX_ANCHORS: usize = 8;

/// The variables an operating system or a language runtime provides to a process.
///
/// A project that reads `PATH` is not asking for something the project defines,
/// and a report that told it to document `HOME` would be a finding about SURE's
/// list rather than about the project. Every name here is one a process is given
/// before any project code runs.
///
/// The list is deliberately **exact and short**. A prefix rule would be
/// unpredictable — a reader asking why `GITHUB_SHA` is set aside and
/// `GITHUB_TOKEN` is not has no way to find out from a name — and a long list is
/// a list of findings SURE has decided not to make. `PROGRAMFILES(X86)` is here
/// because Windows spells it that way; it is not identifier-shaped, which is
/// exactly why [`provided_by_runtime`] compares whole names rather than
/// searching for them.
pub const PROVIDED_BY_RUNTIME: &[&str] = &[
    // The operating system, on every platform.
    "PATH",
    "HOME",
    "PWD",
    "OLDPWD",
    "SHELL",
    "USER",
    "LOGNAME",
    "HOSTNAME",
    "TMPDIR",
    "TEMP",
    "TMP",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "TZ",
    "EDITOR",
    "VISUAL",
    "PAGER",
    "OS",
    // Windows.
    "USERPROFILE",
    "USERNAME",
    "COMPUTERNAME",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "PROCESSOR_ARCHITECTURE",
    // Node.
    "NODE_ENV",
    "NODE_OPTIONS",
    "NODE_PATH",
    // Rust.
    "RUST_BACKTRACE",
    "RUST_LOG",
    "RUSTUP_HOME",
    "CARGO_HOME",
    // Python.
    "PYTHONPATH",
    "PYTHONHOME",
    "PYTHONUNBUFFERED",
    "PYTHONDONTWRITEBYTECODE",
    "VIRTUAL_ENV",
    // Build and integration machines.
    "CI",
    "GITHUB_ACTIONS",
    "GITHUB_WORKSPACE",
    "GITHUB_SHA",
    "GITHUB_REF",
    "GITHUB_REPOSITORY",
    "GITHUB_RUN_ID",
    "GITHUB_SERVER_URL",
];

/// Whether SURE treats this key as the machine's rather than the project's.
///
/// **Exact and case-insensitive.** Exact, because `PATH_TO_DATA` is a project's
/// own key and `NODE_ENVIRONMENT` is not `NODE_ENV`; a substring rule would set
/// both aside. Case-insensitive, because Windows gives one environment to every
/// process regardless of the case a program asks in, and SURE cannot tell `Path`
/// from `PATH` there — the two spellings mean the same variable on one platform
/// and two variables on another, which is a difference no name can settle. Where
/// the two rules could disagree the answer is *set aside*, because the cost of
/// setting aside a key a project did define is one finding not made, and the
/// cost of the other answer would be every Windows run reporting `Path`.
///
/// Deliberately not case-folding with `to_lowercase`: that allocates, and its
/// result depends on a locale's idea of case. `eq_ignore_ascii_case` is the
/// comparison this list is made of.
#[must_use]
pub fn provided_by_runtime(key: &str) -> bool {
    PROVIDED_BY_RUNTIME
        .iter()
        .any(|known| known.eq_ignore_ascii_case(key))
}

/// One key the two sides disagree about, as a claim SURE has taken on.
///
/// A claim exists only for a key the two sides treat differently. A key that is
/// both read and declared makes no claim — there is nothing to settle — and
/// [`CompletenessReport::references`] is where a caller sees those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyClaim {
    key: String,
    status: KeyStatus,
}

impl KeyClaim {
    /// The key, exactly as the project spelled it.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Which side is missing.
    #[must_use]
    pub const fn status(&self) -> KeyStatus {
        self.status
    }

    /// Whether this is the machine's variable rather than the project's.
    ///
    /// Derived from the key every time rather than stored, so a stored flag and
    /// [`provided_by_runtime`] cannot come to different answers about one name.
    #[must_use]
    pub fn is_set_aside(&self) -> bool {
        self.status == KeyStatus::ReadButNotDeclared && provided_by_runtime(&self.key)
    }
}

/// A claim, SURE's verdict on it, and what the verdict rests on.
///
/// The same shape as [`crate::setup::Assessed`], and for the same reason: the
/// four are built together and read together, so there is no constructor that
/// could produce a verdict with no reason or evidence with no claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessedKey {
    claim: KeyClaim,
    assessment: ClaimAssessment,
    severity: Severity,
    reason: String,
    evidence: Vec<Evidence>,
}

impl AssessedKey {
    /// The claim.
    #[must_use]
    pub const fn claim(&self) -> &KeyClaim {
        &self.claim
    }

    /// The key, exactly as the project spelled it.
    #[must_use]
    pub fn key(&self) -> &str {
        self.claim.key()
    }

    /// Which side is missing.
    #[must_use]
    pub const fn status(&self) -> KeyStatus {
        self.claim.status()
    }

    /// SURE's verdict.
    #[must_use]
    pub const fn assessment(&self) -> ClaimAssessment {
        self.assessment
    }

    /// How much the finding is worth, when there is a finding.
    ///
    /// **Nothing here is [`Severity::MustFix`].** That severity blocks hand-off
    /// on its own, and no missing line of configuration documentation is worth
    /// stopping a hand-off for: the project runs without it and the cost is paid
    /// by whoever sets the project up next. The three levels used are
    /// [`Severity::ShouldFixFirst`] for a key the project asks for and names
    /// nowhere, [`Severity::CanFixLater`] for a key its examples name and no
    /// source file reads, and [`Severity::Note`] for a variable the machine
    /// provides.
    ///
    /// **A claim SURE could not settle carries a severity too**, and
    /// deliberately: it is what the finding would be worth if the reading had
    /// finished, which is what a caller deciding whether to go and look needs.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Why, in SURE's own words.
    ///
    /// **The sentence names what was read.** A reader who disagrees with a
    /// verdict has the file and the line to look at, and a reader who agrees
    /// with one has the file and the line to go and fix. What it may contain is
    /// a key name, a path and a count; what it may not contain is a value or a
    /// project's line, and there is no path by which one could reach it.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// What the verdict rests on.
    ///
    /// **Empty for a claim SURE could not settle**, and that is the honest
    /// answer rather than a gap: an anchor says *this is where the claim was
    /// read*, and a claim that was not settled was not read anywhere. Empty for
    /// a [`CannotConfirm`](ClaimAssessment::CannotConfirm) claim means *SURE did
    /// not finish reading*, and [`Self::reason`] says what it did not read.
    #[must_use]
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    /// Whether this is the machine's variable rather than the project's.
    #[must_use]
    pub fn is_set_aside(&self) -> bool {
        self.claim.is_set_aside()
    }

    /// One line for a report.
    ///
    /// The key first, because a list of these is scanned for a name a reader
    /// already has in mind, then the verdict, then the reason.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "{}: {}. {}",
            self.key(),
            self.assessment.label(),
            self.reason
        )
    }
}

/// Everything SURE could settle about one project's configuration keys.
///
/// Built by [`Self::of`], which reads the project and settles what it found in
/// one call, so that a caller cannot settle one project's keys against another
/// project's files.
///
/// The reading is held rather than summarised: [`Self::references`] is the
/// [`ReferenceReport`] every claim was built from, including the keys the two
/// sides agreed about, and a caller that wants the read sites behind a claim
/// asks for them there rather than finding a second copy here that could drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletenessReport {
    root: PathBuf,
    references: ReferenceReport,
    claims: Vec<AssessedKey>,
}

impl CompletenessReport {
    /// Read a project and settle the keys its two sides disagree about.
    ///
    /// `project_fingerprint` is the state this reading is a reading *of*, taken
    /// as a parameter for the reason [`crate::setup::SetupReport::of`] takes it:
    /// evidence that is not bound to a state can never support anything, and a
    /// caller that has no state to bind to has no verdict to record either.
    #[must_use]
    pub fn of(discovery: &Discovery, project_fingerprint: &FingerprintId) -> Self {
        Self::with_options(discovery, &ReferenceOptions::default(), project_fingerprint)
    }

    /// The same, under stated limits.
    ///
    /// The limits decide whether the reading finishes, and the reading decides
    /// whether any claim has a verdict — so a caller that tightens them is
    /// choosing [`ClaimAssessment::CannotConfirm`] for whatever falls outside.
    /// The tests use that deliberately.
    #[must_use]
    pub fn with_options(
        discovery: &Discovery,
        options: &ReferenceOptions,
        project_fingerprint: &FingerprintId,
    ) -> Self {
        let references = ReferenceReport::with_options(discovery, options);
        let complete = references.is_complete();
        let unread = references.unread();
        let claims = references
            .keys()
            .iter()
            .filter_map(|key| assess(key, complete, unread, project_fingerprint))
            .collect();

        Self {
            root: discovery.root.clone(),
            references,
            claims,
        }
    }

    /// The project root the paths are relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The reading every claim was built from.
    ///
    /// This is where a caller finds the keys the two sides agreed about, what
    /// each key's reads were, and what the pass did not read.
    #[must_use]
    pub const fn references(&self) -> &ReferenceReport {
        &self.references
    }

    /// Every claim, in key order.
    #[must_use]
    pub fn claims(&self) -> &[AssessedKey] {
        &self.claims
    }

    /// The claims SURE reached one particular verdict on.
    pub fn with_assessment(
        &self,
        assessment: ClaimAssessment,
    ) -> impl Iterator<Item = &AssessedKey> {
        self.claims
            .iter()
            .filter(move |claim| claim.assessment == assessment)
    }

    /// The claims about one side of the comparison.
    pub fn with_status(&self, status: KeyStatus) -> impl Iterator<Item = &AssessedKey> {
        self.claims
            .iter()
            .filter(move |claim| claim.status() == status)
    }

    /// The claims about keys the project owes something for.
    ///
    /// Every claim that is not [`Self::set_aside`]: a key the project asks for
    /// and names nowhere, or one its examples name and no source file reads.
    pub fn findings(&self) -> impl Iterator<Item = &AssessedKey> {
        self.claims.iter().filter(|claim| !claim.is_set_aside())
    }

    /// The claims about variables the machine provides.
    ///
    /// Not a finding, and in the report anyway: a reader who expects `NODE_ENV`
    /// to be listed somewhere can find it listed here, with the reason SURE did
    /// not count it against the project.
    pub fn set_aside(&self) -> impl Iterator<Item = &AssessedKey> {
        self.claims.iter().filter(|claim| claim.is_set_aside())
    }

    /// How many claims reached each of the four assessments.
    ///
    /// Every assessment is in the list, including the ones no claim reached, so
    /// that a renderer cannot show *"0 contradicted"* by forgetting a row. On
    /// this check that row is always zero and is still printed, for that reason:
    /// a reader who has seen another check's output should not have to infer
    /// from an absent line that nothing was refuted.
    #[must_use]
    pub fn counts(&self) -> Vec<(ClaimAssessment, usize)> {
        ClaimAssessment::ALL
            .iter()
            .map(|assessment| (*assessment, self.with_assessment(*assessment).count()))
            .collect()
    }

    /// Whether SURE finished reading every file a claim could come from.
    ///
    /// **It is the verdicts question and not a separate one**, which is why it
    /// delegates rather than re-deriving: no claim here reaches
    /// [`ClaimAssessment::Confirmed`] unless this is true, and every claim's own
    /// reason carries the answer when it is false.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.references.is_complete()
    }

    /// Every file the reading meant to open and did not.
    #[must_use]
    pub fn unread(&self) -> &[Unread] {
        self.references.unread()
    }

    /// One sentence for a person, counting both sides and what they disagreed on.
    ///
    /// **The counts come first because the sentence has to answer a question a
    /// list of claims cannot.** A report with no claims is two very different
    /// things — a project whose keys all match, and a pass that found no keys at
    /// all — and a sentence that led with *"no disagreements"* would read the
    /// same for both. This one says how many keys SURE read and how many it
    /// found named, so a project with none of either says so.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let read = self
            .references
            .keys()
            .iter()
            .filter(|key| key.is_read())
            .count();
        let declared = self
            .references
            .keys()
            .iter()
            .filter(|key| key.is_declared())
            .count();
        let asked_for = self
            .with_status(KeyStatus::ReadButNotDeclared)
            .filter(|claim| !claim.is_set_aside())
            .count();
        let named = self.with_status(KeyStatus::DeclaredButNotRead).count();
        let provided = self.set_aside().count();

        let mut sentence = format!(
            "SURE read {read} environment or configuration {} asked for in this project's source \
             files and {declared} named in its examples or documents: {}, {}, {}",
            if read == 1 { "key" } else { "keys" },
            count_of(asked_for, "asked for and named nowhere"),
            count_of(named, "named and asked for nowhere"),
            count_of(
                provided,
                "provided by the machine rather than by the project"
            ),
        );
        if !self.is_complete() {
            sentence.push_str(
                ", and SURE did not finish reading the project, so what it did not find may be \
                 what it did not look at",
            );
        }
        sentence.push('.');
        sentence
    }
}

/// `1 key`, `2 keys`, and the clause that says which keys they are.
fn count_of(count: usize, clause: &str) -> String {
    format!(
        "{count} {} {clause}",
        if count == 1 { "key" } else { "keys" }
    )
}

/// Settle one key, or nothing at all when the two sides agree.
///
/// `complete` is [`ReferenceReport::is_complete`] read once for the whole
/// report, because it is a property of the reading and not of a key, and the
/// unread list comes with it because a reason that says *SURE did not finish*
/// without saying **what** it did not read is a sentence a reader cannot act on.
/// Both are facts of the reading rather than of the key, which is why they are
/// parameters rather than something this function looks up.
fn assess(
    key: &KeyReport,
    complete: bool,
    unread: &[Unread],
    project_fingerprint: &FingerprintId,
) -> Option<AssessedKey> {
    // A key both sides agree about makes no claim. Returning nothing is what
    // makes the count of claims a count of *disagreements* rather than a count
    // of keys, and it is done here rather than by a filter at the call site so
    // that a caller cannot forget it.
    //
    // **The tail `match` answers the same way, and either guard alone would
    // do.** It has to: `KeyStatus` has three variants, so that match needs a
    // third arm whether or not this one runs first, and the alternative —
    // building a claim here and dropping it there — is this same decision moved
    // later and paid for on every key the two sides agree about. So the two are
    // one decision written twice rather than a rule and its backstop, which is
    // what the first pass of `target/tmp/p4t006-mutations.py` measured: `m5`
    // removed this guard on its own and the whole crate's suite stayed green,
    // because the arm below caught the case anyway. `m5` is anchored at the
    // answer instead — an agreed key sent into the declared side — which is a
    // mutation that can be caught, and is.
    let status = match key.status {
        KeyStatus::ReadAndDeclared => return None,
        one_sided => one_sided,
    };
    let claim = KeyClaim {
        key: key.name.clone(),
        status,
    };
    let severity = severity_of(status, &key.name);

    if !complete {
        return Some(AssessedKey {
            claim,
            assessment: ClaimAssessment::CannotConfirm,
            severity,
            reason: format!(
                "SURE did not finish reading this project — {} — so it cannot say whether the \
                 other side of this comparison is anywhere in it.",
                what_went_unread(unread)
            ),
            evidence: Vec::new(),
        });
    }

    match status {
        KeyStatus::ReadButNotDeclared => {
            let where_read = key.first_read().map_or_else(
                || "A source file asks for this key".to_owned(),
                |reference| {
                    format!(
                        "`{}` line {} asks for this key",
                        display_path(&reference.path),
                        reference.line
                    )
                },
            );
            let reason = if claim.is_set_aside() {
                format!(
                    "{where_read}, and no example file or document in this project names it. `{}` \
                     is a variable the operating system or a language runtime provides to a \
                     process before any project code runs, so SURE does not count it as \
                     documentation this project owes.",
                    key.name
                )
            } else {
                format!(
                    "{where_read}, and no example file or document in this project names it. SURE \
                     read every file it set out to read."
                )
            };
            Some(AssessedKey {
                claim,
                assessment: ClaimAssessment::Confirmed,
                severity,
                reason: with_anchor_count(reason, key.reads.len()),
                evidence: anchors_of_reads(key, project_fingerprint),
            })
        }
        KeyStatus::DeclaredButNotRead => {
            let where_named = key.declarations.first().map_or_else(
                || "An example file or document in this project names this key".to_owned(),
                |declaration| {
                    format!(
                        "`{}` line {} names this key",
                        display_path(&declaration.path),
                        declaration.line
                    )
                },
            );
            Some(AssessedKey {
                claim,
                assessment: ClaimAssessment::Confirmed,
                severity,
                reason: with_anchor_count(
                    format!(
                        "{where_named}, and no source file SURE read asks for it. A key can be \
                         read by a shell script, a container file, a build configuration or a \
                         language SURE does not read, so this is what SURE read rather than a \
                         statement that the key is unused."
                    ),
                    key.declarations.len(),
                ),
                evidence: anchors_of_declarations(key, project_fingerprint),
            })
        }
        // The other half of the pair described at the top of this function, and
        // not a second rule: an agreed key reached the return above, so this
        // arm is unreachable through `assess` as it is written. It is here
        // because the match must name all three variants, and it is written as
        // an arm rather than as a panic for the reason `references.rs` gives at
        // its own unreachable arm: a claim that should not exist is better left
        // unmade than stopped on.
        KeyStatus::ReadAndDeclared => None,
    }
}

/// What the finding would be worth, from the side that is missing.
fn severity_of(status: KeyStatus, name: &str) -> Severity {
    match status {
        KeyStatus::ReadButNotDeclared if provided_by_runtime(name) => Severity::Note,
        KeyStatus::ReadButNotDeclared => Severity::ShouldFixFirst,
        KeyStatus::DeclaredButNotRead => Severity::CanFixLater,
        // Not reached: `assess` makes no claim about an agreed key, so no
        // severity is ever asked for one. `Note` is the answer that claims the
        // least about a key nothing has been said about.
        KeyStatus::ReadAndDeclared => Severity::Note,
    }
}

/// Why the reading did not finish, in words, for a claim's reason.
///
/// The two ways are read off [`ReferenceReport`]'s public surface and not
/// guessed at. [`ReferenceReport::is_complete`] is false when a file went unread
/// **or** when the walk underneath could not enter part of the project; an empty
/// [`ReferenceReport::unread`] therefore means the walk, and that is the only
/// reading this function makes. The caller has already asked `is_complete` and
/// found it false, so the two cases here are the whole of what is left.
fn what_went_unread(unread: &[Unread]) -> String {
    match unread.first() {
        Some(first) => {
            let files = if unread.len() == 1 {
                "a file it meant to read was not read".to_owned()
            } else {
                format!("{} files it meant to read were not read", unread.len())
            };
            let extra = unread.len() - 1;
            format!(
                "{files}, the first being `{}`{}",
                display_path(&first.path),
                if extra == 0 {
                    String::new()
                } else {
                    format!(" and {extra} more")
                }
            )
        }
        // No unread file and still incomplete: the walk, which is the only
        // other thing `is_complete` weighs.
        None => "the walk over the project could not enter every directory in it".to_owned(),
    }
}

/// The reason with a sentence about a cut evidence list, when it was cut.
fn with_anchor_count(reason: String, sites: usize) -> String {
    if sites <= MAX_ANCHORS {
        return reason;
    }
    format!(
        "{reason} SURE found {sites} places on this side of the comparison and anchors the first \
         {MAX_ANCHORS}."
    )
}

/// One anchor per place a source file asks for a key, up to [`MAX_ANCHORS`].
///
/// **No excerpt.** The line a key is read on is the line a value would be on,
/// and an anchor that carried it could carry a credential. See the module docs.
fn anchors_of_reads(key: &KeyReport, project_fingerprint: &FingerprintId) -> Vec<Evidence> {
    let severity = severity_of(key.status, &key.name);
    key.reads
        .iter()
        .take(MAX_ANCHORS)
        .map(|reference| {
            let shown = display_path(&reference.path);
            Evidence::new(
                // An observed fact rather than a deterministic check: what SURE
                // did was read a file. Nothing here ran anything.
                EvidenceClass::ObservedFact,
                format!(
                    "`{shown}` asks for the key `{}` on line {}.",
                    key.name, reference.line
                ),
                EvidenceAnchor::new(
                    AnchorSubject::LineRange,
                    shown,
                    format!(
                        "the key `{}` asked for on line {}",
                        key.name, reference.line
                    ),
                ),
                Some(project_fingerprint.clone()),
                severity,
            )
        })
        .collect()
}

/// One anchor per place a declaring file names a key, up to [`MAX_ANCHORS`].
fn anchors_of_declarations(key: &KeyReport, project_fingerprint: &FingerprintId) -> Vec<Evidence> {
    let severity = severity_of(key.status, &key.name);
    key.declarations
        .iter()
        .take(MAX_ANCHORS)
        .map(|declaration| {
            let shown = display_path(&declaration.path);
            Evidence::new(
                EvidenceClass::ObservedFact,
                format!(
                    "`{shown}` names the key `{}` on line {}.",
                    key.name, declaration.line
                ),
                EvidenceAnchor::new(
                    AnchorSubject::Documentation,
                    shown,
                    format!("the key `{}` named on line {}", key.name, declaration.line),
                ),
                Some(project_fingerprint.clone()),
                severity,
            )
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A key report, as the reading would have handed it over.
    fn key_report(name: &str, status: KeyStatus, reads: usize, declarations: usize) -> KeyReport {
        use crate::references::{Declaration, DeclarationKind, ReadForm, Reference};
        KeyReport {
            name: name.to_owned(),
            reads: (0..reads)
                .map(|index| Reference {
                    name: name.to_owned(),
                    form: ReadForm::NodeProperty,
                    path: PathBuf::from("src/app.ts"),
                    line: index + 1,
                })
                .collect(),
            declarations: (0..declarations)
                .map(|index| Declaration {
                    name: name.to_owned(),
                    kind: DeclarationKind::Example,
                    path: PathBuf::from(".env.example"),
                    line: index + 1,
                })
                .collect(),
            status,
        }
    }

    #[test]
    fn the_list_is_the_machines_variables_and_not_a_project_s() {
        for known in ["PATH", "HOME", "NODE_ENV", "CI", "RUST_LOG", "VIRTUAL_ENV"] {
            assert!(provided_by_runtime(known), "{known} should be set aside");
        }
        // A longer name that starts like one, names a project owns, and one that
        // differs by a letter.
        for project in [
            "PATH_TO_DATA",
            "HOMEPAGE_URL",
            "NODE_ENVIRONMENT",
            "CITY",
            "PIN",
            "CLIENT_ID",
            "GITHUB_TOKEN",
        ] {
            assert!(
                !provided_by_runtime(project),
                "{project} is the project's and must be reported"
            );
        }
    }

    #[test]
    fn the_case_of_a_name_does_not_decide_the_answer() {
        // Windows gives one environment to every process whatever case a
        // program asks in, so `Path` and `PATH` are one variable there and no
        // name can say which platform it was read on.
        assert!(provided_by_runtime("Path"));
        assert!(provided_by_runtime("path"));
        assert!(provided_by_runtime("node_env"));
    }

    #[test]
    fn the_list_holds_only_names_an_environment_can_have() {
        // Every name is one a process can be given — letters, digits and
        // underscores, except for the two Windows spellings that carry
        // parentheses. A name with a space in it could never match a key, so an
        // entry like that would be a line of the list that does nothing.
        for name in PROVIDED_BY_RUNTIME {
            assert!(!name.is_empty(), "an empty name matches nothing");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '(' || c == ')'),
                "{name} is not a name an environment can hold"
            );
        }
        // No duplicates, which `provided_by_runtime` would tolerate and a reader
        // of the constant would not.
        let mut sorted = PROVIDED_BY_RUNTIME.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            PROVIDED_BY_RUNTIME.len(),
            "a name is listed twice"
        );
    }

    #[test]
    fn a_set_aside_claim_says_so_and_a_projects_own_does_not() {
        let set_aside = KeyClaim {
            key: "NODE_ENV".to_owned(),
            status: KeyStatus::ReadButNotDeclared,
        };
        assert!(set_aside.is_set_aside());
        let own = KeyClaim {
            key: "DATABASE_URL".to_owned(),
            status: KeyStatus::ReadButNotDeclared,
        };
        assert!(!own.is_set_aside());
        // The status decides as well as the name: a variable the machine
        // provides and a document names is on the other side of the comparison
        // and is not set aside.
        let declared = KeyClaim {
            key: "NODE_ENV".to_owned(),
            status: KeyStatus::DeclaredButNotRead,
        };
        assert!(!declared.is_set_aside());
    }

    #[test]
    fn an_agreed_key_makes_no_claim() {
        let agreed = key_report("DATABASE_URL", KeyStatus::ReadAndDeclared, 1, 1);
        assert!(assess(&agreed, true, &[], &fingerprint()).is_none());
    }

    #[test]
    fn the_severity_follows_the_side_that_is_missing() {
        use Severity::{CanFixLater, Note, ShouldFixFirst};
        assert_eq!(
            severity_of(KeyStatus::ReadButNotDeclared, "DATABASE_URL"),
            ShouldFixFirst
        );
        assert_eq!(severity_of(KeyStatus::ReadButNotDeclared, "PATH"), Note);
        assert_eq!(
            severity_of(KeyStatus::DeclaredButNotRead, "LEGACY_TOKEN"),
            CanFixLater
        );
        // Nothing this check can find is severe enough to block a hand-off.
        for status in [KeyStatus::ReadButNotDeclared, KeyStatus::DeclaredButNotRead] {
            for name in ["DATABASE_URL", "PATH", "LEGACY_TOKEN"] {
                assert!(
                    !severity_of(status, name).blocks_hand_off(),
                    "{name} must not block a hand-off"
                );
            }
        }
    }

    #[test]
    fn the_reason_says_how_many_places_were_found_when_the_list_was_cut() {
        assert_eq!(with_anchor_count("A.".to_owned(), 1), "A.");
        assert_eq!(with_anchor_count("A.".to_owned(), MAX_ANCHORS), "A.");
        let cut = with_anchor_count("A.".to_owned(), MAX_ANCHORS + 1);
        assert!(cut.contains(&(MAX_ANCHORS + 1).to_string()), "{cut}");
        assert!(cut.contains(&MAX_ANCHORS.to_string()), "{cut}");
    }

    #[test]
    fn a_long_list_of_reads_is_cut_and_says_so() {
        let key = key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 30, 0);
        let assessed = assess(&key, true, &[], &fingerprint()).unwrap();
        assert_eq!(assessed.evidence().len(), MAX_ANCHORS);
        assert!(assessed.reason().contains("30"), "{}", assessed.reason());
    }

    /// A fingerprint to bind evidence to.
    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    /// One file a reading meant to open and did not.
    fn unread_file(path: &str) -> Unread {
        Unread {
            path: PathBuf::from(path),
            reason: crate::references::UnreadReason::OutOfBudget { limit: 1 },
        }
    }

    #[test]
    fn an_unfinished_reading_leaves_a_claim_with_no_verdict_and_no_evidence() {
        let key = key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 2, 0);
        let unread = [unread_file("src/app.ts")];
        let assessed = assess(&key, false, &unread, &fingerprint()).unwrap();
        assert_eq!(assessed.assessment(), ClaimAssessment::CannotConfirm);
        assert!(
            assessed.evidence().is_empty(),
            "a claim with no verdict has nothing to anchor"
        );
        assert!(
            assessed.reason().contains("src/app.ts"),
            "{}",
            assessed.reason()
        );
        assert!(
            assessed.reason().contains("did not finish"),
            "{}",
            assessed.reason()
        );
    }

    #[test]
    fn an_unfinished_walk_is_told_apart_from_an_unread_file() {
        // `is_complete` weighs two things, and a reason that named a file when
        // no file went unread would send a reader looking for the wrong thing.
        let key = key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 1, 0);
        let assessed = assess(&key, false, &[], &fingerprint()).unwrap();
        assert!(assessed.reason().contains("walk"), "{}", assessed.reason());
        assert!(
            !assessed.reason().contains("the first being"),
            "{}",
            assessed.reason()
        );
    }

    #[test]
    fn a_finished_reading_confirms_the_claim_as_it_is_worded() {
        let key = key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 2, 0);
        let assessed = assess(&key, true, &[], &fingerprint()).unwrap();
        assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
        assert_eq!(assessed.evidence().len(), 2);
        assert!(
            assessed.reason().contains("`src/app.ts` line 1"),
            "{}",
            assessed.reason()
        );
    }

    #[test]
    fn the_other_side_is_confirmed_with_its_limit_in_the_sentence() {
        let key = key_report("LEGACY_TOKEN", KeyStatus::DeclaredButNotRead, 0, 1);
        let assessed = assess(&key, true, &[], &fingerprint()).unwrap();
        assert_eq!(assessed.assessment(), ClaimAssessment::Confirmed);
        assert!(
            assessed
                .reason()
                .contains("rather than a statement that the key is unused"),
            "{}",
            assessed.reason()
        );
    }

    #[test]
    fn a_claim_is_bound_to_the_state_it_was_read_against() {
        let key = key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 1, 0);
        let fingerprint = fingerprint();
        let assessed = assess(&key, true, &[], &fingerprint).unwrap();
        for evidence in assessed.evidence() {
            assert!(
                evidence.is_fresh_for(&fingerprint),
                "evidence that is not bound to a state supports nothing"
            );
            assert_eq!(evidence.fingerprint.as_ref(), Some(&fingerprint));
        }
    }

    #[test]
    fn every_anchor_is_empty_of_excerpts() {
        // The one field of an anchor that exists to hold project text, and the
        // line a key is read on is the line its value is on.
        for key in [
            key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 3, 0),
            key_report("LEGACY_TOKEN", KeyStatus::DeclaredButNotRead, 0, 3),
        ] {
            let assessed = assess(&key, true, &[], &fingerprint()).unwrap();
            assert!(!assessed.evidence().is_empty(), "nothing was anchored");
            for evidence in assessed.evidence() {
                assert!(evidence.anchor.excerpt.is_empty(), "{:?}", evidence);
                assert!(evidence.anchor.is_checkable(), "{:?}", evidence);
            }
        }
    }

    #[test]
    fn a_sentence_holds_the_key_and_no_project_text() {
        // A reason is a constant with a key name and a path in it, and a key
        // name can only be an identifier — so no key a project can write adds a
        // line to a message. The hostile shapes are the ones a project would use
        // if it could.
        let key = key_report("DATABASE_URL", KeyStatus::ReadButNotDeclared, 1, 0);
        let assessed = assess(&key, true, &[], &fingerprint()).unwrap();
        let line = assessed.plain_description();
        assert!(!line.contains('\n'), "{line}");
        assert!(!line.contains('\r'), "{line}");
        assert!(line.starts_with("DATABASE_URL: Confirmed."), "{line}");
    }

    #[test]
    fn a_claim_never_reaches_contradicted() {
        for status in [KeyStatus::ReadButNotDeclared, KeyStatus::DeclaredButNotRead] {
            for complete in [true, false] {
                let key = key_report("DATABASE_URL", status, 1, 1);
                let assessed = assess(&key, complete, &[], &fingerprint()).unwrap();
                assert_ne!(assessed.assessment(), ClaimAssessment::Contradicted);
                // And the reason never says a project was wrong, because SURE
                // has nothing of the project's to be wrong about here.
                assert!(!assessed.reason().contains("does not have"));
            }
        }
    }

    #[test]
    fn a_reason_counts_a_key_the_way_a_reader_reads_it() {
        assert_eq!(
            count_of(0, "asked for and named nowhere"),
            "0 keys asked for and named nowhere"
        );
        assert_eq!(
            count_of(1, "asked for and named nowhere"),
            "1 key asked for and named nowhere"
        );
        assert_eq!(
            count_of(2, "asked for and named nowhere"),
            "2 keys asked for and named nowhere"
        );
    }
}

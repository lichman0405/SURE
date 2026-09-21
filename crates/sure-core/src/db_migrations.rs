//! The shape of a project's database, and the record of how it got that shape.
//!
//! Step 7 of `docs/architecture/CHECK_PIPELINE.md` is completeness analysis, and
//! *missing migrations* is one of the things it names. This is that check. The
//! scenario it answers is the one `fixtures/adversarial/missing-migration`
//! describes — *"Schema/model changed but database migration is missing"* — and
//! it is a release-blocking case in `evaluation/acceptance-manifest.json`.
//!
//! # The pair this check is about
//!
//! A project that manages a database with a migration tool keeps two things: a
//! **shape**, written down in a file the tool reads, and a **record**, a
//! directory of migrations saying how the database reached that shape. The two
//! have to agree, and SURE can see when one of them is not there at all.
//!
//! [`Detector`] is one framework's answer to *where does this framework keep
//! each of those*, and [`DETECTORS`] is the table of them. A detector is in play
//! when one of its files is in the project, answered from the walk and from no
//! manifest at all — the section on reading files rather than manifests says why.
//!
//! # The rule that decides every verdict
//!
//! **A gap is a claim only where the shape is and the record is not.** So
//! [`MigrationsReport::of`] asks one question per detector and answers it against
//! the walk:
//!
//! - **The record holds a migration** — no claim at all. The two agree, which is
//!   the ordinary case, and a report that raised a finding for every project
//!   whose migrations are in order would be a report nobody reads.
//!   [`MigrationsReport::survey`] is where a caller sees those.
//! - **The record is there and holds nothing** — [`ClaimAssessment::Confirmed`]
//!   at [`Severity::MustFix`]. The framework's own record is in the project, so
//!   the project is demonstrably using migrations, and it is empty while a shape
//!   is written down beside it. Nothing else explains that state.
//! - **The record is not there** — [`ClaimAssessment::Confirmed`] at
//!   [`Severity::ShouldFixFirst`], and **not** `MustFix`. The next section is
//!   about why, and it is the decision in this file most worth disagreeing with.
//! - **The reading did not finish** — every claim becomes
//!   [`ClaimAssessment::CannotConfirm`], carries **no evidence at all**, and its
//!   reason names what went unread. This is not decoration: an empty record is
//!   read off a walk, and a walk that stopped early makes a full directory look
//!   like an empty one. [`crate::scan::SkipReason::TooDeep`] and
//!   [`crate::scan::SkipReason::OutOfBudget`] are losses, so
//!   [`crate::discover::Discovery::is_complete`] is false whenever that could have
//!   happened, and a verdict is withheld rather than guessed.
//!
//! # Why "not there" is not `MustFix`, and what that costs
//!
//! A project whose record was *lost* and a project that has not run its first
//! migration yet look **identical** from a file listing. `prisma init` writes a
//! schema and no `prisma/migrations`; a project that has not needed a database
//! change yet is not defective. SURE has read no database, no deployment
//! configuration and no Git history here, so it cannot tell those apart, and a
//! `MustFix` that fired on every freshly scaffolded project would be a false
//! positive of exactly the kind this product exists to prevent.
//!
//! `FROZEN_SEMANTICS.md` defines `MustFix` as the severity that blocks a hand-off
//! *on its own*. A finding whose own sentence has to add "this may be a project
//! that has simply not needed a migration yet" is not that, and giving it that
//! severity would make the vocabulary mean less.
//!
//! **What that costs is stated rather than hidden.** The corpus case
//! `missing-migration` expects `must_fix`, and this check reaches that severity
//! for the state where the framework's record is present and empty. A fixture app
//! that showed the defect by omitting the record *directory* would be reported at
//! `ShouldFixFirst` instead. The two shapes are both "a schema with no migration
//! beside it", so the fixture and this check have to agree on which one
//! `must_fix` names: `progress/DECISIONS.md` records the choice, what it would
//! cost to change, and who has to know. The integration test holds both shapes,
//! so the difference is executable rather than a paragraph.
//!
//! # What it does not do
//!
//! **It does not read a schema or a migration.** Every claim is the walk's own
//! answer about which paths are in the project. No file is opened, so no
//! migration's SQL and no schema's contents reach a sentence, an anchor or an
//! excerpt — `every_anchor_is_empty_of_excerpts` holds the last of those.
//!
//! **It cannot see a change.** The corpus scenario says *changed*, and nothing in
//! a path listing records that. What this check reports is the coarser and
//! checkable fact: a project that writes down the shape of its database and keeps
//! no record of how it got there. A project whose migrations *exist* but do not
//! cover a model added last week is the finer form of the same defect and is
//! **not** detectable here — that needs a schema and its migrations parsed and
//! compared, which is a per-framework reader this task does not build.
//!
//! **It does not verify its own table.** [`DETECTORS`] is SURE's own reading of
//! where each framework keeps its two files, in the same sense that
//! [`crate::env_completeness::PROVIDED_BY_RUNTIME`] is SURE's own reading of what
//! an operating system provides. The rows were written from the frameworks'
//! conventions rather than checked against their documentation, which this build
//! cannot reach, so a row can be wrong — and that is affordable because a row is
//! one line of data and the table is public. A project that keeps its migrations
//! somewhere SURE does not look gets no claim, which is the safe direction: a
//! missed finding rather than an invented one.
//!
//! # Why nothing consults a manifest
//!
//! [`Detector`] has no column naming the packages that declare a framework, and
//! that is a measurement rather than an omission. Discovery recognises a package
//! only when it is in that ecosystem's `TOOLS` table, and none of the frameworks
//! here is: `declares("prisma")`, `declares("drizzle")` and `declares("diesel")`
//! answer `false` in this build whatever the manifest says, because no such row
//! exists. A declaration column would therefore be a field that never fires.
//!
//! Reading the files instead is also the stronger test for the case this product
//! is for. An AI that writes `prisma/schema.prisma` and never adds the package to
//! `package.json` has still left a project whose database shape is managed and
//! unrecorded, and a check that consulted the manifest would miss it.

use std::path::{Path, PathBuf};

use sure_domain::evidence::{
    AnchorSubject, ClaimAssessment, Evidence, EvidenceAnchor, EvidenceClass,
};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

use crate::discover::{Discovery, lookup_key};
use crate::paths::CaseSensitivity;
use crate::scan::{Entry, Skipped, display_path};

/// One framework's answer to where a project keeps its shape and its record.
///
/// The fields are public because the table is a data table and a caller adding a
/// row should not have to read a constructor to know what the columns mean. Two
/// of the four are lists of paths that mean different things, and named fields
/// are what keeps them from being swapped at a call site.
///
/// This is the plug-in point the task's first acceptance criterion names. See
/// [`MigrationsReport::with_detectors`] for the same machinery reaching a detector
/// that is not in [`DETECTORS`].
///
/// **Not `PartialEq`.** One field is a function pointer, and comparing those is
/// not a meaningful question — the compiler has a lint for it, because two
/// different functions can share an address. A caller that wants to know whether
/// two detectors are the same framework compares [`Self::framework`], which is
/// the name a person would use.
#[derive(Debug, Clone, Copy)]
pub struct Detector {
    /// What SURE calls this framework in a sentence.
    ///
    /// The framework's own spelling, so that a reader who knows the project
    /// recognises it. It is a constant in [`DETECTORS`] and never project text.
    pub framework: &'static str,
    /// Paths whose presence in the project means this framework is in use.
    ///
    /// **Exact paths relative to the project root**, not patterns. There is no
    /// glob here on purpose: [`crate::scan`] has one level of `*` expansion and no
    /// more, and a second, weaker path language invented for this table would be a
    /// second thing to get wrong.
    ///
    /// Every path here must be one that names this framework and nothing else.
    /// `src/schema.rs` is a Diesel convention and is **not** in the row, because a
    /// great many Rust projects have a file by that name and none of them is a
    /// database — a marker that fires on ordinary code is how a check starts
    /// reporting on projects it has not understood.
    pub found_by: &'static [&'static str],
    /// Where this framework keeps the record of how the database reached its
    /// shape.
    ///
    /// The first one present in the project is the one looked at. When none is
    /// present the first is the one the claim names, because a sentence has to say
    /// where SURE looked.
    pub record_at: &'static [&'static str],
    /// Whether an entry under [`Self::record_at`] is one migration.
    ///
    /// A function rather than a flag, and the frameworks are why: an Alembic
    /// revision is a `.py` file, a Diesel or Prisma migration is a *directory*, and
    /// a Drizzle one is a `.sql` file. A flag would be a partial description of
    /// three conventions plus a match on the framework's name at the point of use,
    /// which is the shape this table exists to avoid.
    pub counts: fn(&Entry) -> bool,
}

/// Whether an entry is a directory.
///
/// For a framework that writes one directory per migration and puts the SQL
/// inside it. Prisma and Diesel both do, which is why this is a named function
/// rather than a closure written twice.
#[must_use]
pub fn a_migration_directory(entry: &Entry) -> bool {
    entry.kind.is_directory()
}

/// Whether an entry is a Python file.
///
/// For Alembic, which writes one `.py` per revision. **Any** `.py` file under the
/// versions directory counts, including a `__init__.py` or a generated helper —
/// which is what the module documentation means by the table being SURE's own
/// reading rather than a verified one. The cost of being wrong that way is a
/// missed finding, not an invented one.
#[must_use]
pub fn a_python_revision(entry: &Entry) -> bool {
    a_file_ending_in(entry, "py")
}

/// Whether an entry is a SQL file.
///
/// For a framework that writes one `.sql` per migration. Drizzle does.
#[must_use]
pub fn a_sql_migration(entry: &Entry) -> bool {
    a_file_ending_in(entry, "sql")
}

/// Whether an entry is a file whose extension is `extension`, whatever its case.
///
/// A file's *name* is matched the way the platform compares names, but an
/// extension is a convention about a file's kind rather than part of its identity,
/// and a `.SQL` file is a migration wherever it is.
fn a_file_ending_in(entry: &Entry, extension: &str) -> bool {
    if !entry.kind.is_file() {
        return false;
    }
    entry
        .path
        .extension()
        .and_then(|found| found.to_str())
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

/// Every framework SURE knows where to look for.
///
/// Four rows, and the shortness is deliberate in the way
/// [`crate::env_completeness::PROVIDED_BY_RUNTIME`]'s is: a row is a claim about
/// another project's conventions, and a long table is a long list of ways to be
/// wrong about somebody's repository. Each row here names a file that is this
/// framework's own and nobody else's.
///
/// The frameworks that are *not* here are as much a part of the design. Django,
/// SQLAlchemy, TypeORM, Sequelize and Knex keep their models across many files at
/// no fixed path, so there is no shape to look beside the record and no row that
/// would be true. SQLx, Flyway and Goose read their shape from a live database
/// rather than a file, so the same. Ruby on Rails does keep a single
/// `db/schema.rb`, and is out for a different reason: Ruby is not an ecosystem
/// [`crate::discover`] reads, and a check about a language SURE cannot otherwise
/// see would be a claim out of nowhere.
pub const DETECTORS: &[Detector] = &[
    Detector {
        framework: "Prisma",
        found_by: &["prisma/schema.prisma"],
        record_at: &["prisma/migrations"],
        // One directory per migration, each holding a `migration.sql`. The
        // `migration_lock.toml` beside them is a file, and is not one.
        counts: a_migration_directory,
    },
    Detector {
        framework: "Drizzle",
        found_by: &[
            "drizzle.config.ts",
            "drizzle.config.mts",
            "drizzle.config.js",
            "drizzle.config.mjs",
        ],
        record_at: &["drizzle"],
        // `drizzle-kit generate` writes `<timestamp>_<name>.sql` into the
        // configured `out`, which is `./drizzle` unless a project says otherwise.
        // A project that says otherwise is the miss this row accepts; see the
        // module documentation.
        counts: a_sql_migration,
    },
    Detector {
        framework: "Alembic",
        found_by: &["alembic.ini"],
        record_at: &["alembic/versions"],
        // One `.py` per revision under the `script_location` that `alembic.ini`
        // names, which is `alembic` unless a project says otherwise.
        counts: a_python_revision,
    },
    Detector {
        framework: "Diesel",
        found_by: &["diesel.toml"],
        record_at: &["migrations"],
        // `diesel migration generate` makes one directory per migration with
        // `up.sql` and `down.sql` inside.
        counts: a_migration_directory,
    },
];

/// What was at the place a framework keeps its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Record {
    /// The place is in the project and holds this many migrations.
    Holds(usize),
    /// The place is in the project and holds none.
    Empty,
    /// The place is not in the project.
    Absent,
}

impl Record {
    /// Whether the record says how the database reached its shape.
    #[must_use]
    pub const fn holds_a_migration(self) -> bool {
        matches!(self, Self::Holds(_))
    }
}

/// One framework found in the project, and what was at its record.
///
/// This is the reading, and it settles nothing: a look exists for every detector
/// in play whether or not there is anything to say about it, so that a caller
/// asking *did SURE check my migrations* gets an answer for the projects whose
/// migrations are in order as well as for the ones where they are not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Looked {
    framework: &'static str,
    shape: PathBuf,
    record_at: PathBuf,
    record: Record,
}

impl Looked {
    /// What SURE calls the framework.
    #[must_use]
    pub const fn framework(&self) -> &'static str {
        self.framework
    }

    /// The file that says this framework is in use, relative to the root.
    #[must_use]
    pub fn shape(&self) -> &Path {
        &self.shape
    }

    /// Where the record was looked for, relative to the root.
    ///
    /// Named even when the record is [`Record::Absent`], because a reason sentence
    /// has to say where SURE looked and a reader has to be able to go and look
    /// there.
    #[must_use]
    pub fn record_at(&self) -> &Path {
        &self.record_at
    }

    /// What was there.
    #[must_use]
    pub const fn record(&self) -> Record {
        self.record
    }

    /// The gap this look is a claim about, or nothing at all.
    ///
    /// **The one place the decision is written.** A record holding a migration
    /// means the two agree and there is no claim; the other two states are the
    /// claim. Returning `None` here rather than filtering at the call site is what
    /// makes [`MigrationsReport::claims`] a list of *gaps* rather than a list of
    /// looks, and it is one decision rather than two that agree — which is what the
    /// equivalent-mutant finding in `P4-T006` was about.
    fn gap(&self) -> Option<SchemaGap> {
        match self.record {
            Record::Holds(_) => None,
            Record::Empty | Record::Absent => Some(SchemaGap {
                framework: self.framework,
                shape: self.shape.clone(),
                record_at: self.record_at.clone(),
                record: self.record,
            }),
        }
    }
}

/// A shape written down with no record of how the database reached it.
///
/// A claim exists only where the two halves disagree. Every framework whose record
/// holds a migration makes none, and [`Survey::looked`] is where a caller sees
/// those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaGap {
    framework: &'static str,
    shape: PathBuf,
    record_at: PathBuf,
    record: Record,
}

impl SchemaGap {
    /// What SURE calls the framework.
    #[must_use]
    pub const fn framework(&self) -> &'static str {
        self.framework
    }

    /// The file that says this framework is in use.
    #[must_use]
    pub fn shape(&self) -> &Path {
        &self.shape
    }

    /// Where the record should be and is not, or is and is empty.
    #[must_use]
    pub fn record_at(&self) -> &Path {
        &self.record_at
    }

    /// Which of the two states the record is in.
    #[must_use]
    pub const fn record(&self) -> Record {
        self.record
    }

    /// How much the gap is worth.
    ///
    /// [`Severity::MustFix`] where the record is there and empty, and
    /// [`Severity::ShouldFixFirst`] where it is absent. The module documentation
    /// argues both, and the short version is that an empty record beside a shape
    /// has one explanation while a missing one has two, only one of which is a
    /// defect.
    ///
    /// [`Record::Holds`] is not reachable here — [`Looked::gap`] makes no claim for
    /// it — and is written as an arm rather than a panic for the reason
    /// [`crate::references`] gives at its own unreachable arm: a claim that should
    /// not exist is better left unmade than stopped on.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self.record {
            Record::Empty => Severity::MustFix,
            Record::Absent => Severity::ShouldFixFirst,
            Record::Holds(_) => Severity::Note,
        }
    }
}

/// A claim, SURE's verdict on it, and what the verdict rests on.
///
/// The same shape as [`crate::env_completeness::AssessedKey`], and for the same
/// reason: the four are built together and read together, so there is no
/// constructor that could produce a verdict with no reason, or evidence with no
/// claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessedGap {
    claim: SchemaGap,
    assessment: ClaimAssessment,
    severity: Severity,
    reason: String,
    evidence: Vec<Evidence>,
}

impl AssessedGap {
    /// The claim.
    #[must_use]
    pub const fn claim(&self) -> &SchemaGap {
        &self.claim
    }

    /// What SURE calls the framework.
    #[must_use]
    pub const fn framework(&self) -> &'static str {
        self.claim.framework
    }

    /// Which of the two states the record is in.
    #[must_use]
    pub const fn record(&self) -> Record {
        self.claim.record
    }

    /// SURE's verdict.
    #[must_use]
    pub const fn assessment(&self) -> ClaimAssessment {
        self.assessment
    }

    /// How much the gap is worth, when there is a gap.
    ///
    /// **A claim SURE could not settle carries a severity too**, and deliberately:
    /// it is what the finding would be worth if the reading had finished, which is
    /// what a caller deciding whether to go and look at the project needs.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Why, in SURE's own words.
    ///
    /// **The sentence names what was read.** A reader who disagrees with it has the
    /// two paths to go and look at. What it may contain is a framework name, a path
    /// SURE measured and a count; what it may not contain is project text or a
    /// migration's SQL, and none of that is opened to reach it.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// What the verdict rests on.
    ///
    /// **Empty for a claim SURE could not settle**, which is the honest answer
    /// rather than a gap: an anchor says *this is where the claim was read*, and a
    /// claim that was not settled was not read anywhere. For a
    /// [`CannotConfirm`](ClaimAssessment::CannotConfirm) claim, empty means SURE did
    /// not finish reading, and [`Self::reason`] says what it did not read.
    #[must_use]
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    /// One line for a report.
    ///
    /// The framework first, because a list of these is scanned for a name a reader
    /// already has in mind, then the verdict, then the reason.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "{}: {}. {}",
            self.framework(),
            self.assessment.label(),
            self.reason
        )
    }
}

/// The reading half: which frameworks are in play and what is at each record.
///
/// Usable on its own, and it is the half the task's first acceptance criterion is
/// about — a caller that wants to know what SURE found without a verdict takes
/// this and none of the sentences. It does not live in a module of its own the way
/// [`crate::references`] does, because its only consumer in this build is the
/// report below; the split happens when a second one appears, not before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Survey {
    root: PathBuf,
    looked: Vec<Looked>,
    complete: bool,
    losses: Vec<Skipped>,
}

impl Survey {
    /// Look over a project for every framework in `detectors`.
    #[must_use]
    pub fn of(discovery: &Discovery, detectors: &[Detector]) -> Self {
        let entries = discovery.scan.entries();
        let looked = detectors
            .iter()
            .filter_map(|detector| look(entries, detector))
            .collect();

        Self {
            root: discovery.root.clone(),
            looked,
            complete: discovery.is_complete(),
            losses: discovery.losses().cloned().collect(),
        }
    }

    /// The project root the paths are relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every framework in play, in [`DETECTORS`] order.
    ///
    /// A framework whose files are not in the project is not here, so an empty
    /// list means *no framework SURE knows about left a file in this project*
    /// rather than *this project has no database*.
    #[must_use]
    pub fn looked(&self) -> &[Looked] {
        &self.looked
    }

    /// Whether the walk this rests on saw everything.
    ///
    /// `false` is the honest answer to a great many questions and not a failure:
    /// it means part of the project was not looked at, so a record that is not in
    /// the survey may exist rather than be absent.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    /// Everything the walk could have hidden a record in.
    ///
    /// Only losses — a directory that could not be listed, a level the depth limit
    /// stopped, the point the entry budget ran out. A declared skip like
    /// `node_modules` is not a loss and is not here.
    #[must_use]
    pub fn losses(&self) -> &[Skipped] {
        &self.losses
    }
}

/// Everything SURE could settle about one project's database record.
///
/// Built by [`Self::of`], which reads the project and settles what it found in one
/// call, so that a caller cannot settle one project's records against another
/// project's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationsReport {
    survey: Survey,
    claims: Vec<AssessedGap>,
}

impl MigrationsReport {
    /// Look over a project for the frameworks in [`DETECTORS`] and settle them.
    ///
    /// `project_fingerprint` is the state this reading is a reading *of*, taken as
    /// a parameter for the reason
    /// [`crate::env_completeness::CompletenessReport::of`] takes it: evidence that
    /// is not bound to a state can never support anything, and a caller that has no
    /// state to bind to has no verdict to record either.
    #[must_use]
    pub fn of(discovery: &Discovery, project_fingerprint: &FingerprintId) -> Self {
        Self::with_detectors(discovery, DETECTORS, project_fingerprint)
    }

    /// The same, for a table a caller supplies.
    ///
    /// **This is what makes a detector pluggable.** Nothing in this module reads
    /// [`DETECTORS`] except [`Self::of`]; everything below takes the table it was
    /// given, so a framework SURE does not ship is a [`Detector`] value at the call
    /// site and no change to any file here.
    #[must_use]
    pub fn with_detectors(
        discovery: &Discovery,
        detectors: &[Detector],
        project_fingerprint: &FingerprintId,
    ) -> Self {
        let survey = Survey::of(discovery, detectors);
        let complete = survey.complete;
        let losses = survey.losses.clone();
        let claims = survey
            .looked
            .iter()
            .filter_map(|looked| assess(looked, complete, &losses, project_fingerprint))
            .collect();

        Self { survey, claims }
    }

    /// The project root the paths are relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.survey.root()
    }

    /// The reading every claim was built from.
    ///
    /// This is where a caller finds the frameworks whose record is in order, what
    /// each record was, and what the walk did not look at.
    #[must_use]
    pub const fn survey(&self) -> &Survey {
        &self.survey
    }

    /// Every claim, in [`DETECTORS`] order.
    #[must_use]
    pub fn claims(&self) -> &[AssessedGap] {
        &self.claims
    }

    /// The claims SURE reached one particular verdict on.
    pub fn with_assessment(
        &self,
        assessment: ClaimAssessment,
    ) -> impl Iterator<Item = &AssessedGap> {
        self.claims
            .iter()
            .filter(move |claim| claim.assessment == assessment)
    }

    /// The claims about one state of the record.
    pub fn with_record(&self, record: Record) -> impl Iterator<Item = &AssessedGap> {
        self.claims
            .iter()
            .filter(move |claim| claim.record() == record)
    }

    /// How many claims reached each of the four assessments.
    ///
    /// Every assessment is in the list, including the ones no claim reached, so
    /// that a renderer cannot show *"0 contradicted"* by forgetting a row. On this
    /// check that row is always zero and is still printed, for that reason: a reader
    /// who has seen another check's output should not have to infer from an absent
    /// line that nothing was refuted.
    #[must_use]
    pub fn counts(&self) -> Vec<(ClaimAssessment, usize)> {
        ClaimAssessment::ALL
            .iter()
            .map(|assessment| (*assessment, self.with_assessment(*assessment).count()))
            .collect()
    }

    /// Whether SURE finished reading every part of the project a claim could come
    /// from.
    ///
    /// **It is the verdicts question and not a separate one**, which is why it
    /// delegates rather than re-deriving: no claim here reaches
    /// [`ClaimAssessment::Confirmed`] unless this is true, and every claim's own
    /// reason carries the answer when it is false.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.survey.is_complete()
    }

    /// What the walk did not look at.
    #[must_use]
    pub fn losses(&self) -> &[Skipped] {
        self.survey.losses()
    }

    /// One sentence for a person, counting what was looked at and what was found.
    ///
    /// **The count of frameworks comes first because the sentence has to answer a
    /// question a list of claims cannot.** A report with no claims is two very
    /// different things — a project whose migrations are in order, and a project
    /// where SURE recognised no database framework at all — and a sentence that led
    /// with *"no gaps"* would read the same for both. This one says how many
    /// frameworks SURE looked at, so the second project says so.
    #[must_use]
    pub fn plain_description(&self) -> String {
        let looked = self.survey.looked().len();
        let empty = self.with_record(Record::Empty).count();
        let absent = self.with_record(Record::Absent).count();

        let mut sentence = format!(
            "SURE looked for how {looked} database {} this project keeps files for record the shape \
             of its database: {}, {}",
            if looked == 1 {
                "framework"
            } else {
                "frameworks"
            },
            count_of(empty, "with a record that is there and holds nothing"),
            count_of(absent, "with no record at all"),
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

/// `1 framework with …`, `2 frameworks with …`.
fn count_of(count: usize, clause: &str) -> String {
    format!(
        "{count} {} {clause}",
        if count == 1 {
            "framework"
        } else {
            "frameworks"
        }
    )
}

/// Look for one framework's two halves, or nothing when it is not in play.
///
/// A detector is in play when one of its own files is in the project. Which one was
/// found is kept rather than the whole list, because a sentence naming four paths
/// where one was found would be longer without saying more.
///
/// The comparison goes through [`lookup_key`] against
/// [`CaseSensitivity::platform`], which is the rule for *is this the same file*
/// rather than the rule for *is this the same name*. Discovery is given a case rule
/// for its ignore tables — a decision about comparing two names — but this lookup
/// asks whether the file a framework's convention names is there, and on Windows
/// `prisma/schema.prisma` and `Prisma/Schema.prisma` are one file whatever a caller
/// told the ignore tables to think.
fn look(entries: &[Entry], detector: &Detector) -> Option<Looked> {
    let case = CaseSensitivity::platform();
    let shape = detector
        .found_by
        .iter()
        .find(|path| has(entries, path, case))?;

    // The first of the framework's places that is in the project, or the first one,
    // named for a sentence that has to say where SURE looked. `or_else` rather than
    // an index, so a row with an empty list makes no claim instead of stopping the
    // pass — the table's own test holds every shipped row to a non-empty list, and
    // this is what happens to one that is not.
    let (record_at, record) = detector
        .record_at
        .iter()
        .find_map(|path| {
            has(entries, path, case).then(|| {
                (
                    PathBuf::from(path),
                    count_under(entries, path, detector.counts, case),
                )
            })
        })
        .or_else(|| {
            detector
                .record_at
                .first()
                .map(|path| (PathBuf::from(path), Record::Absent))
        })?;

    Some(Looked {
        framework: detector.framework,
        shape: PathBuf::from(shape),
        record_at,
        record,
    })
}

/// Whether the walk found this path, as a file or as a directory.
///
/// Both count: `prisma/schema.prisma` is a file and `prisma/migrations` is a
/// directory, and a project that has a *file* where a migrations directory belongs
/// has something at the name rather than nothing — which is what this answers, and
/// what the claim is worded around.
fn has(entries: &[Entry], path: &str, case: CaseSensitivity) -> bool {
    let wanted = lookup_key(Path::new(path), case);
    entries
        .iter()
        .any(|entry| lookup_key(&entry.path, case) == wanted)
}

/// How many migrations are under a directory the walk found.
///
/// **Read off the walk, never off the filesystem.** Every entry a scan lists under
/// a directory it entered is in the scan, so this is the whole answer for a
/// complete walk — and a walk that stopped inside the directory recorded a loss,
/// which is why [`Survey::is_complete`] is false there and no verdict is given.
/// That pairing is what stops a directory full of migrations from being read as an
/// empty one; `a_directory_the_walk_did_not_finish_reading_is_not_reported_as_empty`
/// holds it.
///
/// The separator after the prefix matters: `prisma/migrations-old/2020_x` starts
/// with `prisma/migrations` as a string and is not inside it as a path.
fn count_under(
    entries: &[Entry],
    at: &str,
    counts: fn(&Entry) -> bool,
    case: CaseSensitivity,
) -> Record {
    let under = format!("{}/", lookup_key(Path::new(at), case));
    let held = entries
        .iter()
        .filter(|entry| lookup_key(&entry.path, case).starts_with(&under) && counts(entry))
        .count();

    if held == 0 {
        Record::Empty
    } else {
        Record::Holds(held)
    }
}

/// Settle one look, or nothing at all when the record holds a migration.
///
/// `complete` is [`Survey::is_complete`] read once for the whole report, because it
/// is a property of the walk and not of a framework, and the losses come with it
/// because a reason that says *SURE did not finish* without saying **what** it did
/// not read is a sentence a reader cannot act on. Both are facts of the reading
/// rather than of the framework, which is why they are parameters rather than
/// something this function looks up.
fn assess(
    looked: &Looked,
    complete: bool,
    losses: &[Skipped],
    project_fingerprint: &FingerprintId,
) -> Option<AssessedGap> {
    let claim = looked.gap()?;
    let severity = claim.severity();
    let shape = display_path(&claim.shape);
    let record_at = display_path(&claim.record_at);

    if !complete {
        return Some(AssessedGap {
            claim,
            assessment: ClaimAssessment::CannotConfirm,
            severity,
            reason: format!(
                "SURE did not finish reading this project — {} — so it cannot say whether \
                 `{record_at}`, where {} keeps the record of how the database reached its shape, \
                 is anywhere in it.",
                what_went_unread(losses),
                looked.framework
            ),
            evidence: Vec::new(),
        });
    }

    let reason = match claim.record {
        Record::Empty => format!(
            "`{shape}` says {} is in use in this project, and `{record_at}`, where {} keeps the \
             record of how the database reached its shape, is here and holds no migration. The \
             framework's own record is in the project, so this project uses migrations, and \
             nothing in it says how the database got the shape the project gives it. SURE read \
             every file it set out to read.",
            looked.framework, looked.framework
        ),
        // The two states SURE cannot tell apart, named in the sentence rather than
        // left for the reader to work out — see the module documentation.
        Record::Absent => format!(
            "`{shape}` says {} is in use in this project, and `{record_at}`, where {} keeps the \
             record of how the database reached its shape, is not in it. A project that has not \
             written its first migration yet and one whose record was lost or never made look the \
             same from here, so this is what SURE read rather than a statement that a migration is \
             owed. SURE read every file it set out to read.",
            looked.framework, looked.framework
        ),
        // Not reached: `Looked::gap` makes no claim about a record that holds a
        // migration, so no reason is ever asked for one.
        Record::Holds(_) => String::new(),
    };

    Some(AssessedGap {
        claim: claim.clone(),
        assessment: ClaimAssessment::Confirmed,
        severity,
        reason,
        evidence: evidence_of(&claim, project_fingerprint),
    })
}

/// What the verdict rests on: the file that names the framework, and the place the
/// record was looked for.
///
/// **No excerpt.** Nothing here opened a file, so there is no project text to quote
/// — a weaker promise than redaction can make, because there is nothing to redact.
///
/// Both anchors are [`AnchorSubject::Database`], which is what that variant is for
/// and what it has been unused for until this check: an anchor on a schema and an
/// anchor on a migrations directory are the two things it names.
#[must_use]
fn evidence_of(claim: &SchemaGap, project_fingerprint: &FingerprintId) -> Vec<Evidence> {
    let severity = claim.severity();
    let shape = display_path(&claim.shape);
    let record_at = display_path(&claim.record_at);

    let at_record = match claim.record {
        Record::Empty => format!("`{record_at}` is in this project and holds no migration."),
        Record::Absent => format!("`{record_at}` is not in this project."),
        // Not reached, as in `assess`: no claim is made about a record that holds a
        // migration.
        Record::Holds(held) => format!("`{record_at}` holds {held} migrations."),
    };

    vec![
        Evidence::new(
            // An observed fact rather than a deterministic check: what SURE did was
            // look at the project. Nothing here ran anything.
            EvidenceClass::ObservedFact,
            format!("`{shape}` is in this project."),
            EvidenceAnchor::new(
                AnchorSubject::Database,
                shape,
                format!("the file that says {} is in use here", claim.framework),
            ),
            Some(project_fingerprint.clone()),
            severity,
        ),
        Evidence::new(
            EvidenceClass::ObservedFact,
            at_record,
            EvidenceAnchor::new(
                AnchorSubject::Database,
                record_at,
                format!("where {} keeps its migration record", claim.framework),
            ),
            Some(project_fingerprint.clone()),
            severity,
        ),
    ]
}

/// Why the reading did not finish, in words, for a claim's reason.
///
/// [`Survey::is_complete`] is `false` exactly when there is a skip that loses
/// coverage, so the first arm is the reachable one and the second is what happens
/// if that ever stops being true — a sentence that would rather say less than the
/// wrong place.
fn what_went_unread(losses: &[Skipped]) -> String {
    match losses.first() {
        Some(first) => {
            let places = if losses.len() == 1 {
                "1 place SURE meant to look at was not looked at".to_owned()
            } else {
                format!(
                    "{} places SURE meant to look at were not looked at",
                    losses.len()
                )
            };
            let extra = losses.len() - 1;
            format!(
                "{places}, the first being `{}`{}",
                display_path(&first.path),
                if extra == 0 {
                    String::new()
                } else {
                    format!(" and {extra} more")
                }
            )
        }
        None => "the walk over the project could not enter every directory in it".to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::scan::EntryKind;

    /// The pieces these tests are built from: a list of entries, which is what
    /// every rule here reads.
    ///
    /// Not a [`crate::scan::Scan`], because a scan is only ever made by walking a
    /// real directory — it has no constructor and no field a test can set, which is
    /// deliberate on its part. The rules below take the entry list rather than the
    /// scan for exactly this reason: what they read is a list of what was found, so
    /// a fixture that states one is the honest way to test them, and
    /// `tests/db_migrations.rs` is where the same rules are driven over real
    /// directories.
    ///
    /// **A fixture has to list the directories as well as the files**, because a
    /// walk lists every directory it enters — including one it finds empty, which
    /// is the only way an empty migrations directory is visible at all. A fixture
    /// that named only files would be describing a project no walk produces, and
    /// the rule that looks for where a framework keeps its record would answer
    /// [`Record::Absent`] about a directory the fixture had just put a file in.
    fn entries_of(paths: &[(&str, EntryKind)]) -> Vec<Entry> {
        paths
            .iter()
            .map(|(path, kind)| entry(path, *kind))
            .collect()
    }

    fn file(path: &str) -> (&str, EntryKind) {
        (path, EntryKind::File)
    }

    fn dir(path: &str) -> (&str, EntryKind) {
        (path, EntryKind::Directory)
    }

    /// One entry, built the way a walk builds one.
    fn entry(path: &str, kind: EntryKind) -> Entry {
        Entry {
            path: PathBuf::from(path),
            kind,
        }
    }

    /// The detector for a framework SURE does not ship, used by the tests that are
    /// about the plug-in point rather than about the table.
    const TESTBOOK: Detector = Detector {
        framework: "Testbook",
        found_by: &["testbook/schema.toml"],
        record_at: &["testbook/revisions"],
        counts: a_migration_directory,
    };

    /// A `Looked`, built the way `look` builds one.
    fn looked(framework: &'static str, record: Record) -> Looked {
        Looked {
            framework,
            shape: PathBuf::from("shape.file"),
            record_at: PathBuf::from("the/record"),
            record,
        }
    }

    fn fingerprint() -> FingerprintId {
        FingerprintId::generate()
    }

    /// A report built from looks a test states, for the tests that are about what a
    /// report says rather than about what a walk found.
    ///
    /// The claims go through the real `assess` rather than being written out, so a
    /// test that passes here is passing about the code that ships.
    fn report_of(looks: Vec<Looked>, complete: bool) -> MigrationsReport {
        let survey = Survey {
            root: PathBuf::from("."),
            looked: looks,
            complete,
            losses: if complete {
                Vec::new()
            } else {
                vec![Skipped {
                    path: PathBuf::from("somewhere"),
                    reason: crate::scan::SkipReason::OutOfBudget,
                    detail: None,
                }]
            },
        };
        let losses = survey.losses.clone();
        let claims = survey
            .looked
            .iter()
            .filter_map(|look| assess(look, complete, &losses, &fingerprint()))
            .collect();
        MigrationsReport { survey, claims }
    }

    #[test]
    fn the_table_is_four_frameworks_and_no_two_share_a_file() {
        assert_eq!(DETECTORS.len(), 4, "the table is the four rows argued for");
        for detector in DETECTORS {
            assert!(
                !detector.framework.is_empty(),
                "a detector has to have a name to be named in a sentence"
            );
            assert!(
                !detector.found_by.is_empty(),
                "{} has nothing that would tell SURE it is in use",
                detector.framework
            );
            assert!(
                !detector.record_at.is_empty(),
                "{} has nowhere to look for its record",
                detector.framework
            );
            for path in detector.found_by {
                assert!(
                    Path::new(path).is_relative() && !path.contains('*'),
                    "`{path}` is not an exact project-relative path"
                );
            }
        }

        // Two detectors sharing a marker file would both be in play for one project,
        // and one project would get two claims about one directory. Distinct names
        // and distinct markers are what keep a look and a framework one to one.
        for (index, detector) in DETECTORS.iter().enumerate() {
            for other in &DETECTORS[index + 1..] {
                assert_ne!(
                    detector.framework, other.framework,
                    "two detectors answer to one name"
                );
                for path in detector.found_by {
                    assert!(
                        !other.found_by.contains(path),
                        "two detectors are in play from `{path}`"
                    );
                }
            }
        }
    }

    #[test]
    fn the_predicates_answer_what_their_names_say() {
        let a_file = entry("migrations/2020_01_01_init/up.sql", EntryKind::File);
        let a_directory = entry("migrations/2020_01_01_init", EntryKind::Directory);

        assert!(a_migration_directory(&a_directory));
        assert!(!a_migration_directory(&a_file));
        assert!(a_sql_migration(&a_file));
        assert!(!a_sql_migration(&a_directory));
        assert!(!a_python_revision(&a_file));

        // The extension is matched without regard to case.
        assert!(a_python_revision(&entry(
            "alembic/versions/abc123_INIT.PY",
            EntryKind::File
        )));

        // A directory called `notes.sql` is not a migration, and neither is a file
        // with no extension.
        assert!(!a_sql_migration(&entry(
            "drizzle/notes.sql",
            EntryKind::Directory
        )));
        assert!(!a_sql_migration(&entry("drizzle/notes", EntryKind::File)));
    }

    #[test]
    fn a_detector_that_is_not_in_play_is_not_looked_at() {
        let entries = entries_of(&[file("src/main.rs")]);

        assert!(look(&entries, &DETECTORS[0]).is_none());
        assert!(look(&entries, &TESTBOOK).is_none());
    }

    #[test]
    fn a_record_that_holds_a_migration_makes_no_claim() {
        let entries = entries_of(&[
            file("alembic.ini"),
            dir("alembic"),
            dir("alembic/versions"),
            file("alembic/versions/abc123_init.py"),
        ]);
        let found = look(&entries, &DETECTORS[2]).expect("Alembic is in use here");

        assert_eq!(found.framework(), "Alembic");
        assert_eq!(found.record(), Record::Holds(1));
        assert!(found.record().holds_a_migration());
        assert!(
            found.gap().is_none(),
            "the two halves agree here, so there is no claim to make"
        );
    }

    #[test]
    fn only_a_migration_directory_is_counted_as_a_prisma_migration() {
        let entries = entries_of(&[
            file("prisma/schema.prisma"),
            dir("prisma/migrations"),
            // The lock file is a file, and is not a migration.
            file("prisma/migrations/migration_lock.toml"),
            dir("prisma/migrations/20240101_init"),
            file("prisma/migrations/20240101_init/migration.sql"),
        ]);
        let found = look(&entries, &DETECTORS[0]).expect("Prisma is in use here");

        assert_eq!(found.record(), Record::Holds(1), "{found:?}");
    }

    #[test]
    fn every_place_a_framework_might_keep_its_record_is_looked_at() {
        // Drizzle's row is the one with four markers, and this holds that the row
        // is read whichever one a project used rather than only the first.
        for marker in [
            "drizzle.config.ts",
            "drizzle.config.mts",
            "drizzle.config.js",
            "drizzle.config.mjs",
        ] {
            let entries =
                entries_of(&[file(marker), dir("drizzle"), file("drizzle/0000_init.sql")]);
            let found = look(&entries, &DETECTORS[1]).expect("Drizzle is in use here");

            assert_eq!(found.shape(), Path::new(marker));
            assert_eq!(found.record(), Record::Holds(1));
        }
    }

    #[test]
    fn a_record_inside_a_similarly_named_directory_is_not_counted_as_inside_it() {
        // `prisma/migrations-old/2020_x` starts with `prisma/migrations` as text and
        // is not inside it as a path.
        let entries = entries_of(&[
            file("prisma/schema.prisma"),
            dir("prisma/migrations"),
            dir("prisma/migrations-old"),
            dir("prisma/migrations-old/2020_x"),
        ]);
        let found = look(&entries, &DETECTORS[0]).expect("Prisma is in use here");

        assert_eq!(found.record(), Record::Empty, "{found:?}");
    }

    #[test]
    fn a_marker_file_that_is_only_a_similar_name_is_not_a_framework() {
        let entries = entries_of(&[
            file("docs/prisma/schema.prisma"),
            file("alembic.ini.bak"),
            file("libs/diesel.toml"),
        ]);

        for detector in DETECTORS {
            assert!(
                look(&entries, detector).is_none(),
                "{} was read as in use from a name that only looks like its own",
                detector.framework
            );
        }
    }

    #[test]
    fn a_detector_sure_does_not_ship_is_used_without_touching_any_shipped_code() {
        // The task's first acceptance criterion, as a test. `TESTBOOK` is a value in
        // this file: the framework is invented, nothing in the module above mentions
        // it, and the pass runs it anyway.
        let entries = entries_of(&[
            file("testbook/schema.toml"),
            dir("testbook"),
            dir("testbook/revisions"),
        ]);

        // The shipped table sees nothing here, which is what makes this a measurement
        // of the plug-in point rather than of the table.
        for detector in DETECTORS {
            assert!(look(&entries, detector).is_none());
        }

        let found = look(&entries, &TESTBOOK).expect("Testbook is in use here");
        assert_eq!(found.framework(), "Testbook");
        assert_eq!(found.record(), Record::Empty);
        assert_eq!(
            found.gap().expect("an empty record is a gap").severity(),
            Severity::MustFix,
            "a plugged-in detector reaches the same verdict a shipped one does"
        );
    }

    #[test]
    fn a_row_with_nowhere_to_look_makes_no_claim_rather_than_stopping_the_pass() {
        const NO_RECORD: Detector = Detector {
            framework: "Hollow",
            found_by: &["hollow.toml"],
            record_at: &[],
            counts: a_migration_directory,
        };
        let entries = entries_of(&[file("hollow.toml")]);

        assert!(look(&entries, &NO_RECORD).is_none());
    }

    #[test]
    fn the_severity_follows_what_is_at_the_record() {
        assert_eq!(
            looked("Alembic", Record::Empty)
                .gap()
                .expect("an empty record is a gap")
                .severity(),
            Severity::MustFix
        );
        assert_eq!(
            looked("Alembic", Record::Absent)
                .gap()
                .expect("an absent record is a gap")
                .severity(),
            Severity::ShouldFixFirst
        );
        // The `Holds` arm is unreachable through `look` — `Looked::gap` makes no
        // claim about a record that holds a migration — so it is reached here by
        // building the claim directly. An unreachable arm is where a default goes
        // wrong without anything noticing, which is why it is held to an answer
        // rather than left as a `panic!`.
        let unreachable = SchemaGap {
            framework: "Alembic",
            shape: PathBuf::from("shape.file"),
            record_at: PathBuf::from("the/record"),
            record: Record::Holds(3),
        };
        assert_eq!(unreachable.severity(), Severity::Note);
    }

    #[test]
    fn a_record_that_is_there_and_empty_is_a_must_fix_that_blocks_a_hand_off() {
        let report = report_of(vec![looked("Alembic", Record::Empty)], true);
        let claim = report.claims().first().expect("one claim");

        assert_eq!(claim.record(), Record::Empty);
        assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
        assert_eq!(claim.severity(), Severity::MustFix);
        assert!(
            claim.severity().blocks_hand_off(),
            "the corpus scenario is release-blocking"
        );
        assert!(claim.reason().contains("holds no migration"));
    }

    #[test]
    fn a_record_that_is_not_there_does_not_block_a_hand_off_on_its_own() {
        let report = report_of(vec![looked("Alembic", Record::Absent)], true);
        let claim = report.claims().first().expect("one claim");

        assert_eq!(claim.record(), Record::Absent);
        assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
        assert_eq!(claim.severity(), Severity::ShouldFixFirst);
        assert!(
            !claim.severity().blocks_hand_off(),
            "a state two things explain must not stop a hand-off on its own"
        );
        assert!(
            claim.reason().contains("look the same from here"),
            "the sentence has to say which two states it cannot tell apart: {}",
            claim.reason()
        );
    }

    #[test]
    fn a_must_fix_is_grounded_the_way_the_release_gate_reads_one() {
        // `Finding::is_grounded` wants a must_fix to carry at least one anchor whose
        // class can support one alone and which a reader can check. This check never
        // builds a `Finding` — that arrives with aggregation — so the question is
        // asked of the evidence directly, with the domain's own predicate rather than
        // a copy of it.
        let report = report_of(vec![looked("Alembic", Record::Empty)], true);
        let claim = report.claims().first().expect("one claim");

        assert!(!claim.evidence().is_empty());
        assert!(
            claim
                .evidence()
                .iter()
                .any(|evidence| evidence.class.can_alone_support_must_fix()
                    && evidence.anchor.is_checkable()),
            "a must_fix nothing can stand behind is the false green this product exists to prevent"
        );
        for evidence in claim.evidence() {
            assert_eq!(evidence.anchor.subject, AnchorSubject::Database);
            assert_eq!(evidence.severity, Severity::MustFix);
        }
    }

    #[test]
    fn every_anchor_is_empty_of_excerpts() {
        for record in [Record::Empty, Record::Absent] {
            let claim = looked("Diesel", record).gap().expect("a gap");
            for evidence in evidence_of(&claim, &fingerprint()) {
                assert!(
                    evidence.anchor.excerpt.is_empty(),
                    "nothing here opened a file, so nothing is quotable: {}",
                    evidence.anchor.excerpt
                );
            }
        }
    }

    #[test]
    fn a_claim_is_bound_to_the_state_it_was_read_against() {
        let fingerprint = fingerprint();
        let claim = looked("Prisma", Record::Empty).gap().expect("a gap");

        for one in evidence_of(&claim, &fingerprint) {
            assert_eq!(
                one.fingerprint.as_ref(),
                Some(&fingerprint),
                "evidence that is not bound to a state can never support anything"
            );
        }
    }

    #[test]
    fn an_unfinished_walk_settles_nothing_and_anchors_nothing() {
        let report = report_of(vec![looked("Prisma", Record::Empty)], false);
        let claim = report.claims().first().expect("one claim");

        assert!(!report.is_complete());
        assert_eq!(claim.assessment(), ClaimAssessment::CannotConfirm);
        assert!(
            claim.evidence().is_empty(),
            "an anchor on a claim SURE could not settle would be evidence for nothing"
        );
        assert_eq!(
            claim.severity(),
            Severity::MustFix,
            "a claim SURE could not settle still says what it would be worth"
        );
        assert!(
            claim.reason().contains("did not finish"),
            "{}",
            claim.reason()
        );
        assert!(
            claim.reason().contains("somewhere"),
            "the reason has to name what was not looked at: {}",
            claim.reason()
        );
    }

    #[test]
    fn the_reason_counts_the_places_not_looked_at_the_way_a_reader_reads_them() {
        let one = vec![Skipped {
            path: PathBuf::from("a"),
            reason: crate::scan::SkipReason::TooDeep,
            detail: None,
        }];
        assert_eq!(
            what_went_unread(&one),
            "1 place SURE meant to look at was not looked at, the first being `a`"
        );

        let three = vec![
            Skipped {
                path: PathBuf::from("a"),
                reason: crate::scan::SkipReason::TooDeep,
                detail: None,
            },
            Skipped {
                path: PathBuf::from("b"),
                reason: crate::scan::SkipReason::Unreadable,
                detail: None,
            },
            Skipped {
                path: PathBuf::from("c"),
                reason: crate::scan::SkipReason::OutOfBudget,
                detail: None,
            },
        ];
        assert_eq!(
            what_went_unread(&three),
            "3 places SURE meant to look at were not looked at, the first being `a` and 2 more"
        );

        // `is_complete` is false exactly when there is a loss, so this arm is
        // unreachable — and it says something true rather than nothing.
        assert!(what_went_unread(&[]).contains("could not enter"));
    }

    #[test]
    fn nothing_is_ever_contradicted() {
        // A contradicted claim is one a project made and SURE refuted. This project
        // made none: an absent record refutes nothing, because no file says where the
        // database's shape came from.
        for record in [Record::Empty, Record::Absent, Record::Holds(2)] {
            let report = report_of(vec![looked("Prisma", record)], true);
            assert_eq!(
                report
                    .with_assessment(ClaimAssessment::Contradicted)
                    .count(),
                0
            );
        }
    }

    #[test]
    fn every_assessment_is_counted_even_at_zero() {
        let report = report_of(vec![looked("Prisma", Record::Empty)], true);
        let counts = report.counts();

        assert_eq!(counts.len(), ClaimAssessment::ALL.len());
        for assessment in ClaimAssessment::ALL {
            assert!(
                counts.iter().any(|(one, _)| one == assessment),
                "{assessment:?} is missing from the table"
            );
        }
        assert_eq!(
            counts
                .iter()
                .find(|(one, _)| *one == ClaimAssessment::Confirmed)
                .map(|(_, count)| *count),
            Some(1)
        );
    }

    #[test]
    fn the_sentence_tells_a_project_with_no_frameworks_from_one_with_no_gaps() {
        let none = report_of(Vec::new(), true);
        let in_order = report_of(vec![looked("Alembic", Record::Holds(4))], true);

        // Both have no claims, and the sentence is what keeps them apart.
        assert!(none.claims().is_empty());
        assert!(in_order.claims().is_empty());
        assert_ne!(none.plain_description(), in_order.plain_description());
        assert!(
            none.plain_description().contains("0 database frameworks"),
            "{}",
            none.plain_description()
        );
        assert!(
            in_order
                .plain_description()
                .contains("1 database framework"),
            "{}",
            in_order.plain_description()
        );
        assert!(none.plain_description().ends_with('.'));
    }

    #[test]
    fn the_sentence_counts_both_states_and_says_when_the_walk_fell_short() {
        let both = report_of(
            vec![
                looked("Prisma", Record::Empty),
                looked("Drizzle", Record::Absent),
            ],
            true,
        );
        let sentence = both.plain_description();

        assert!(sentence.contains("2 database frameworks"), "{sentence}");
        assert!(
            sentence.contains("1 framework with a record that is there and holds nothing"),
            "{sentence}"
        );
        assert!(
            sentence.contains("1 framework with no record at all"),
            "{sentence}"
        );
        assert!(
            !sentence.contains("did not finish"),
            "a complete walk must not carry the warning: {sentence}"
        );

        let short = report_of(vec![looked("Prisma", Record::Empty)], false);
        assert!(
            short.plain_description().contains("did not finish"),
            "{}",
            short.plain_description()
        );
    }

    #[test]
    fn a_sentence_holds_only_a_framework_name_a_path_and_a_count() {
        // The text a project can reach is a path, and a path is what SURE measured.
        // Nothing here quotes a file, so what this holds is that the sentence names
        // the framework, the place it looked and the verdict — and nothing else.
        let report = report_of(
            vec![
                looked("Prisma", Record::Empty),
                looked("Alembic", Record::Absent),
            ],
            true,
        );
        for claim in report.claims() {
            assert!(claim.reason().contains(claim.framework()));
            assert!(
                claim
                    .reason()
                    .contains(&display_path(claim.claim().record_at())),
                "the reason has to name where SURE looked: {}",
                claim.reason()
            );
            assert!(
                claim
                    .plain_description()
                    .contains(claim.assessment().label())
            );
        }
    }

    // The two halves of the platform rule, one test per platform, so that the CI
    // matrix runs both. They are separate tests rather than one with a branch
    // because a single test asserting one thing on Windows and the opposite on Linux
    // would pass while establishing neither.
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn this_platform_reads_a_differently_cased_name_as_the_same_name() {
        // Windows and macOS would open `Prisma/Schema.prisma` when asked for
        // `prisma/schema.prisma`, so a lookup that missed it would report a project
        // as having no schema. The rule comes from `lookup_key`, which is the walk's
        // own.
        let entries = entries_of(&[
            file("Prisma/Schema.prisma"),
            dir("Prisma"),
            dir("Prisma/Migrations"),
        ]);
        let found = look(&entries, &DETECTORS[0]).expect("the same file, spelled differently");

        assert_eq!(found.framework(), "Prisma");
        assert_eq!(found.record(), Record::Empty);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn this_platform_reads_a_differently_cased_name_as_a_different_name() {
        // The same entries on a filesystem where those are two paths. Asserting only
        // the other half would let a change that folded case everywhere pass.
        let entries = entries_of(&[
            file("Prisma/Schema.prisma"),
            dir("Prisma"),
            dir("Prisma/Migrations"),
        ]);

        assert!(
            look(&entries, &DETECTORS[0]).is_none(),
            "there is no `prisma/schema.prisma` on this platform"
        );
    }
}

//! Database schema and migration consistency, end to end.
//!
//! `crate::db_migrations`'s own tests hold the rules over entry lists this file's
//! sibling states by hand; this file drives the same rules over **real
//! directories**, which is where they have to add up to something true about a
//! project — and where `P4-T007`'s two acceptance sentences can be checked:
//!
//! > Framework-specific detectors are pluggable.
//! > Mandatory missing-migration fixture can be detected.
//!
//! # The claim this file exists to hold
//!
//! **"Can be detected" is a claim about a pair of runs, not about one.** A check
//! that answered `must_fix` to every project would pass a test that only asserted
//! the corpus case, so every fixture below is asserted against its own control:
//! the same project with its migration in place makes no claim, and the same
//! project read by a walk that stopped early makes no *verdict*. The three
//! together are what make "detected" mean something.
//!
//! **"Pluggable" is a claim about a file that was not changed.** The test that
//! holds it defines a detector for a framework SURE does not ship, in this file,
//! and drives it over real files. Nothing in `src/` mentions that framework, so a
//! pass here is a measurement of the seam rather than of the table.
//!
//! # The fixture corpus this is not
//!
//! `fixtures/adversarial/missing-migration/` is still a stub, and it is not this
//! task's to fill in: `P14-T001`–`P14-T011` own the runnable fixture applications
//! and the machine-readable expected outcomes, and
//! `crates/sure-protocol/tests/conformance.rs` pins that the fixture-expectation
//! type is absent until then. So the project this file builds lives under
//! `target/tmp` for the length of one test, and the *directory* stays P14's.
//!
//! What this file does owe `P14` is the shape: `the_missing_migration_scenario_…`
//! is the corpus case, named for its id in `evaluation/acceptance-manifest.json`,
//! and it fixes which project shape this check calls `must_fix`. The other shape —
//! a schema with no record directory at all — is asserted separately, at
//! `ShouldFixFirst`, with the reason the two differ written into the module.
//!
//! ## What "changed" costs this check
//!
//! The corpus scenario reads *"Schema/model changed but database migration is
//! missing."* Nothing in a path listing records a change, so what is detected here
//! is the coarser fact that a project writes down the shape of its database and
//! keeps no record of how it got there. A project whose migrations exist but do
//! not cover a model added last week is **not** detected by this check and is a
//! stated limit rather than an oversight — see the module documentation.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because a
// panic is a message nobody chose. A test is the one place they are the point: the
// panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sure_core::db_migrations::{
    DETECTORS, Detector, MigrationsReport, Record, a_migration_directory,
};
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::scan::ScanOptions;
use sure_domain::evidence::{AnchorSubject, ClaimAssessment};
use sure_domain::ids::FingerprintId;
use sure_domain::severity::Severity;

/// A value that must never reach a report.
///
/// Shaped like a credential and placed inside a schema file and a migration, so
/// that a check which grew a reader for either would have something to leak. This
/// check has no reader: every claim it makes is about a *path*, which is what
/// `no_project_text_reaches_a_report` and `the_module_opens_no_file` hold between
/// them.
const CANARY: &str = "sk-live-CANARY-4c1e8a6b2f";

/// A directory under the workspace's git-ignored `target/tmp`.
///
/// The claiming rules live in `sure_testkit::scratch`, and the history that put
/// them there is this repository's: clearing a fixed path with
/// `let _ = remove_dir_all(..)` and then treating it as fresh fails on Windows,
/// and the test then describes a directory that was never emptied. So nothing
/// here is adopted — a directory is taken with `create_dir`, which fails when
/// the name is taken, and one that is already there is skipped rather than
/// entered — and the helper clears only directories carrying *its own*
/// process's id, which no live process can own.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        Self {
            project: sure_testkit::scratch::directory("db migrations", test),
        }
    }

    /// Write a file, making the directories it needs.
    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.project.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    /// Make an empty directory.
    ///
    /// The shape at the centre of this check's `must_fix`: a record that is there
    /// and holds nothing. A directory with nothing in it is invisible to
    /// everything except a walk, which lists the directory itself.
    fn mkdir(&self, relative: &str) -> &Self {
        let full = self.project.join(relative);
        std::fs::create_dir_all(&full)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", full.display()));
        self
    }

    fn discovery(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default()).expect("discover")
    }

    /// The same project read by a walk that is not allowed to go as deep.
    ///
    /// The scan records a [`sure_core::scan::SkipReason::TooDeep`] for every level
    /// it stopped at, so this is the honest way to produce the state the
    /// completeness rule is about: a real project, really walked, with part of it
    /// really not looked at.
    fn discovery_at_depth(&self, max_depth: usize) -> Discovery {
        let options = DiscoverOptions {
            scan: ScanOptions::default().with_max_depth(max_depth),
            ..DiscoverOptions::default()
        };
        discover(&self.project, &options).expect("discover")
    }

    fn report(&self) -> MigrationsReport {
        MigrationsReport::of(&self.discovery(), &FingerprintId::generate())
    }

    fn report_at_depth(&self, max_depth: usize) -> MigrationsReport {
        MigrationsReport::of(
            &self.discovery_at_depth(max_depth),
            &FingerprintId::generate(),
        )
    }
}

/// Everything a report says, as one string, for a canary to be looked for in.
///
/// The `Debug` rendering **and** every sentence a person reads. A report is not a
/// file, so the four surfaces are the two renderings and the two pieces of text
/// inside them.
fn everything_a_report_says(report: &MigrationsReport) -> String {
    let mut text = format!("{report:?}\n");
    text.push_str(&report.plain_description());
    text.push('\n');
    for claim in report.claims() {
        text.push_str(&claim.plain_description());
        text.push('\n');
        text.push_str(claim.reason());
        text.push('\n');
        for evidence in claim.evidence() {
            text.push_str(&evidence.summary);
            text.push('\n');
            text.push_str(&evidence.anchor.location);
            text.push('\n');
            text.push_str(&evidence.anchor.locator);
            text.push('\n');
            text.push_str(&evidence.anchor.excerpt);
            text.push('\n');
        }
    }
    text
}

/// The corpus case, in the one project shape this check calls `must_fix`: a
/// framework's schema file, beside a record directory the framework itself set up
/// and left empty.
///
/// An empty `prisma/migrations` is not a hypothetical — `prisma migrate dev`
/// creates the directory, and a project whose migrations were removed, rolled back
/// or never generated has exactly this. The alternative shape, where the directory
/// is not there at all, is the next test.
fn the_corpus_fixture() -> Fixture {
    let fixture = Fixture::new("missing-migration");
    fixture
        .write(
            "package.json",
            r#"{"name":"shop","dependencies":{"@prisma/client":"^5"}}"#,
        )
        .write("prisma/schema.prisma", "model Order {\n  id Int\n}\n")
        .mkdir("prisma/migrations");
    fixture
}

#[test]
fn the_missing_migration_scenario_is_reported_as_a_must_fix() {
    let report = the_corpus_fixture().report();
    let claims: Vec<_> = report.claims().iter().collect();

    assert_eq!(claims.len(), 1, "{:?}", report.claims());
    let claim = claims[0];
    assert_eq!(claim.framework(), "Prisma");
    assert_eq!(claim.record(), Record::Empty);
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(claim.severity(), Severity::MustFix);
    assert!(
        claim.severity().blocks_hand_off(),
        "the corpus case is release-blocking, so it has to be able to stop a hand-off alone"
    );

    // A `must_fix` that nothing stands behind is the false green this product
    // exists to prevent, so the evidence is held to the domain's own bar for one.
    assert!(!claim.evidence().is_empty());
    assert!(
        claim
            .evidence()
            .iter()
            .any(|evidence| evidence.class.can_alone_support_must_fix()
                && evidence.anchor.is_checkable()),
        "no anchor here can support the severity the claim carries"
    );
    for evidence in claim.evidence() {
        assert_eq!(
            evidence.anchor.subject,
            AnchorSubject::Database,
            "the variant that exists for a schema and a migration is the one to use"
        );
        assert_eq!(evidence.severity, Severity::MustFix);
        assert!(
            evidence.fingerprint.is_some(),
            "evidence not bound to a state can never support anything"
        );
    }

    // And the sentence has to name both halves, because a reader who disagrees
    // with the verdict has to be able to go and look at the two files.
    assert!(
        claim.reason().contains("prisma/schema.prisma"),
        "{}",
        claim.reason()
    );
    assert!(
        claim.reason().contains("prisma/migrations"),
        "{}",
        claim.reason()
    );
}

#[test]
fn the_same_project_with_its_migration_in_place_makes_no_claim() {
    // The control for the test above, and the reason "can be detected" is a
    // statement about two runs: the only difference is one directory inside
    // `prisma/migrations`, and the finding disappears with it.
    let fixture = Fixture::new("migration-in-place");
    fixture
        .write("prisma/schema.prisma", "model Order {\n  id Int\n}\n")
        .write(
            "prisma/migrations/20240101_init/migration.sql",
            "CREATE TABLE \"Order\" (id INT);\n",
        );

    let report = fixture.report();
    assert!(
        report.claims().is_empty(),
        "a project whose migrations are in order got a finding: {:?}",
        report.claims()
    );
    let looked = report.survey().looked();
    assert_eq!(looked.len(), 1);
    assert_eq!(looked[0].framework(), "Prisma");
    assert_eq!(looked[0].record(), Record::Holds(1));

    // The two runs differ in one directory, and this is that directory.
    assert_eq!(
        the_corpus_fixture().report().claims().len(),
        1,
        "the corpus fixture stopped being a finding, so the pair no longer says anything"
    );
}

#[test]
fn a_schema_with_no_record_directory_at_all_is_reported_without_stopping_a_hand_off() {
    // The other shape of "a schema with no migration beside it", and the one this
    // check refuses to call `must_fix`. A freshly scaffolded project looks exactly
    // like this, so the finding is real and its severity says a person should look
    // rather than that nobody may proceed.
    let fixture = Fixture::new("no-record-directory");
    fixture.write("prisma/schema.prisma", "model Order {\n  id Int\n}\n");

    let report = fixture.report();
    let claim = report.claims().first().expect("one claim");

    assert_eq!(claim.record(), Record::Absent);
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(claim.severity(), Severity::ShouldFixFirst);
    assert!(
        !claim.severity().blocks_hand_off(),
        "a state that a new project and a negligent one share must not stop a hand-off"
    );
    assert!(
        claim
            .reason()
            .contains("has not written its first migration yet"),
        "the sentence has to say which two states it cannot tell apart: {}",
        claim.reason()
    );
    assert!(
        claim.reason().contains("prisma/migrations"),
        "{}",
        claim.reason()
    );
    assert!(!claim.evidence().is_empty());
}

#[test]
fn a_walk_that_did_not_finish_reading_does_not_invent_a_missing_migration() {
    // The false-positive guard, over a real project. `prisma/migrations` holds a
    // migration here, and a walk that stopped at the level above it lists the
    // directory and none of its contents — so a check without the completeness
    // rule reads a full record as an empty one and reports `must_fix` on a project
    // whose migrations are exactly right.
    let fixture = Fixture::new("truncated-walk");
    fixture
        .write("prisma/schema.prisma", "model Order {\n  id Int\n}\n")
        .write(
            "prisma/migrations/20240101_init/migration.sql",
            "CREATE TABLE \"Order\" (id INT);\n",
        );

    // The control: read as far as it takes, this project is in order.
    let complete = fixture.report();
    assert!(complete.is_complete());
    assert!(complete.claims().is_empty());

    // The same project, read by a walk that stops one level above the migrations.
    let truncated = fixture.report_at_depth(2);
    assert!(
        !truncated.is_complete(),
        "the walk did not stop where it was told to"
    );
    let claim = truncated.claims().first().expect("one claim");
    assert_eq!(
        claim.record(),
        Record::Empty,
        "the record does read as empty, which is exactly why the verdict matters"
    );
    assert_eq!(
        claim.assessment(),
        ClaimAssessment::CannotConfirm,
        "a full directory read by a truncated walk is not an empty one"
    );
    assert!(
        claim.evidence().is_empty(),
        "an anchor on a claim SURE could not settle would be evidence for nothing"
    );
    assert_eq!(
        claim.severity(),
        Severity::MustFix,
        "the severity says what the finding would be worth, not that it was made"
    );
    assert!(
        claim.reason().contains("did not finish"),
        "{}",
        claim.reason()
    );
    assert!(
        !truncated.losses().is_empty(),
        "the walk forgot to record what it did not look at"
    );

    // Nothing anywhere in the report claims a missing migration as a fact.
    assert_eq!(
        truncated
            .with_assessment(ClaimAssessment::Confirmed)
            .count(),
        0,
        "an unfinished walk settled a claim"
    );
}

#[test]
fn a_project_with_no_database_framework_at_all_makes_no_claim() {
    // The other false positive: an ordinary project that says nothing about a
    // database must not be read as having a database problem.
    let fixture = Fixture::new("no-framework");
    fixture
        .write(
            "package.json",
            r#"{"name":"site","dependencies":{"react":"^18"}}"#,
        )
        .write("src/index.ts", "export const ready = true;\n")
        .write("Cargo.toml", "[package]\nname = \"site\"\n");

    let report = fixture.report();

    assert!(report.claims().is_empty(), "{:?}", report.claims());
    assert!(report.survey().looked().is_empty());
    assert!(report.is_complete());
    // And the sentence says which of the two empty-with-no-claims states this is.
    assert!(
        report.plain_description().contains("0 database frameworks"),
        "{}",
        report.plain_description()
    );
}

#[test]
fn every_shipped_framework_is_read_from_one_project() {
    // Four rows, four different notions of what a migration is — a directory, a
    // SQL file, a Python file, a directory — in one project.
    let fixture = Fixture::new("all-four");
    fixture
        .write("prisma/schema.prisma", "model A {\n  id Int\n}\n")
        .mkdir("prisma/migrations")
        .write("drizzle.config.ts", "export default {};\n")
        .write("drizzle/0000_baseline.sql", "CREATE TABLE a (id INT);\n")
        .write("alembic.ini", "[alembic]\n")
        .write("alembic/versions/abc123_first.py", "revision = 'abc123'\n")
        .write("diesel.toml", "[migrations_directory]\n")
        .mkdir("migrations");

    let report = fixture.report();
    let by_framework: Vec<(&str, Record, Severity)> = report
        .claims()
        .iter()
        .map(|claim| (claim.framework(), claim.record(), claim.severity()))
        .collect();

    // Drizzle and Alembic have a migration, so they make no claim at all; Prisma
    // and Diesel have an empty record, so they do.
    assert_eq!(
        by_framework,
        vec![
            ("Prisma", Record::Empty, Severity::MustFix),
            ("Diesel", Record::Empty, Severity::MustFix),
        ],
        "{by_framework:?}"
    );

    let looked: Vec<&str> = report
        .survey()
        .looked()
        .iter()
        .map(|look| look.framework())
        .collect();
    assert_eq!(
        looked,
        vec!["Prisma", "Drizzle", "Alembic", "Diesel"],
        "every framework in the table is looked at, in the table's own order"
    );
}

#[test]
fn a_framework_sure_does_not_ship_is_detected_over_real_files() {
    // P4-T007's first acceptance criterion, driven end to end. `HALFBAKED` is a
    // detector value in this file for a framework that does not exist; nothing
    // under `src/` names it, and the pass reads its files, counts its record and
    // issues the same verdict a shipped row gets.
    const HALFBAKED: Detector = Detector {
        framework: "Halfbaked",
        found_by: &["halfbaked/shape.toml"],
        record_at: &["halfbaked/applied"],
        counts: a_migration_directory,
    };

    let fixture = Fixture::new("pluggable");
    fixture
        .write("halfbaked/shape.toml", "[tables]\n")
        .mkdir("halfbaked/applied");

    // The shipped table sees nothing here — which is what makes this a measurement
    // of the seam rather than of the table.
    let shipped = fixture.report();
    assert!(shipped.claims().is_empty());
    assert!(shipped.survey().looked().is_empty());
    assert_eq!(DETECTORS.len(), 4, "the shipped table is not the four rows");

    let plugged = MigrationsReport::with_detectors(
        &fixture.discovery(),
        &[HALFBAKED],
        &FingerprintId::generate(),
    );
    let claim = plugged.claims().first().expect("one claim");
    assert_eq!(claim.framework(), "Halfbaked");
    assert_eq!(claim.record(), Record::Empty);
    assert_eq!(claim.assessment(), ClaimAssessment::Confirmed);
    assert_eq!(claim.severity(), Severity::MustFix);
    assert!(
        claim.reason().contains("halfbaked/applied"),
        "{}",
        claim.reason()
    );

    // And the same detector stops claiming once the record holds something.
    let applied = fixture.project.join("halfbaked/applied/0001_init");
    std::fs::create_dir_all(&applied)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", applied.display()));
    let settled = MigrationsReport::with_detectors(
        &fixture.discovery(),
        &[HALFBAKED],
        &FingerprintId::generate(),
    );
    assert!(settled.claims().is_empty(), "{:?}", settled.claims());
}

#[test]
fn no_project_text_reaches_a_report() {
    // `the_module_opens_no_file` holds that nothing here is read. This holds the
    // other half, and it is the half a future change would break: a check that
    // grew a reader and put what it read into a sentence would fail here whatever
    // the source rule said.
    //
    // A canary in a file's contents can only be leaked by reading it, so the test
    // asserts both that the check *did* something — there is a claim — and that
    // the canary is in none of the surfaces a report has.
    let fixture = Fixture::new("canary");
    fixture
        .write(
            "prisma/schema.prisma",
            &format!("// {CANARY}\nmodel Order {{\n  id Int\n}}\n"),
        )
        .write(
            "prisma/migrations/migration_lock.toml",
            &format!("# {CANARY}\nprovider = \"postgresql\"\n"),
        );

    let report = fixture.report();
    let claim = report.claims().first().expect("one claim");
    assert_eq!(
        claim.record(),
        Record::Empty,
        "the lock file is not a migration, which is why this record is empty"
    );

    let everything = everything_a_report_says(&report);
    assert!(
        !everything.contains(CANARY),
        "a value from a project file reached a report"
    );
    // The search can fail, which is what makes the assertion above a measurement
    // rather than a formality: put the canary in and the same function finds it.
    assert!(
        everything_a_report_says(&report).contains("prisma/schema.prisma"),
        "the report does not name the file it looked at, so this test proves nothing"
    );
}

#[test]
fn the_module_opens_no_file() {
    // How "nothing is read" is established rather than promised. A source rule is
    // the only instrument that can answer it: a test that ran the check and saw no
    // leak cannot tell a check that read nothing from one that read and kept
    // quiet.
    //
    // The rule is the repository's own idiom — `tests/check_schedule.rs` reads the
    // sources to hold a rule about them — and it is stated over the part of the
    // file above the test module, because a test may read the repository.
    let text = std::fs::read_to_string(
        sure_testkit::repository_root().join("crates/sure-core/src/db_migrations.rs"),
    )
    .expect("the module this rule is about");

    let (shipped, tests) = text
        .split_once("#[cfg(test)]")
        .expect("the module has a test module, and this rule is about the half above it");
    assert!(
        tests.len() > 1_000,
        "the cut between shipped code and tests is not where this rule thinks it is"
    );
    assert!(
        shipped.len() > 5_000,
        "the shipped half is {} bytes, which is not this module",
        shipped.len()
    );

    for (number, line) in shipped.lines().enumerate() {
        // Prose explains the rule and has to be able to name what it forbids.
        if line.trim_start().starts_with("//") {
            continue;
        }
        for forbidden in [
            "std::fs",
            "fs::read",
            "fs::File",
            "read_to_string",
            "File::open",
            "OpenOptions",
            "BufReader",
        ] {
            assert!(
                !line.contains(forbidden),
                "line {} opens a file (`{forbidden}`), which this check must never do: {line}",
                number + 1
            );
        }
    }
}

// The two halves of the platform rule over a real filesystem, one test per
// platform so that the CI matrix runs both. The unit tests hold the same rule over
// stated entry lists; what these add is the walk's own answer for a name the
// operating system gave back with different case.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn this_platform_reads_a_differently_cased_schema_as_the_same_file() {
    let fixture = Fixture::new("cased-schema");
    fixture
        .write("Prisma/Schema.prisma", "model Order {\n  id Int\n}\n")
        .mkdir("Prisma/Migrations");

    let report = fixture.report();
    let claim = report.claims().first().expect("one claim");

    assert_eq!(claim.framework(), "Prisma");
    assert_eq!(claim.record(), Record::Empty);
}

#[cfg(not(any(windows, target_os = "macos")))]
#[test]
fn this_platform_reads_a_differently_cased_schema_as_a_different_file() {
    let fixture = Fixture::new("cased-schema");
    fixture
        .write("Prisma/Schema.prisma", "model Order {\n  id Int\n}\n")
        .mkdir("Prisma/Migrations");

    let report = fixture.report();

    assert!(
        report.claims().is_empty(),
        "there is no `prisma/schema.prisma` on this platform: {:?}",
        report.claims()
    );
    assert!(report.survey().looked().is_empty());
}

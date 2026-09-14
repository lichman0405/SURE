//! `sure check`, and the one thing it can do in this build.
//!
//! # Why this is a module rather than an arm of `commands.rs`
//!
//! [`Command::report`](crate::commands::Command::report) is a match from a
//! command to a value. Two of its arms look at something — `doctor` at this
//! machine, this one at a project and at the store — and the rule that keeps the
//! match readable is that a report *is* a value: something a test can build and a
//! renderer can read, not a thing that goes and looks. So the looking happens
//! here and the match holds the answer.
//!
//! # What `--goal` does, and what it refuses to claim
//!
//! P2-T010's acceptance is that `sure check` can receive and store a trusted
//! explicit goal **without requiring raw transcript recording**. So a goal typed
//! on the command line is written to the record store as a requirement whose
//! source is [`IntentSource::ExplicitUserGoal`] — the source
//! [`IntentSource::is_user_requirement`] accepts, and one that
//! [`IntentSource::requires_full_recording`] asks nothing of. No opt-in is
//! consulted and no recording is written, because there is no session to capture:
//! the words were handed to SURE in the same breath as the command that stored
//! them.
//!
//! Then the run stops, and the report says so. This build cannot check a project
//! yet, so the answer is not a report about the project — but neither is it a
//! refusal of the kind every other unimplemented command produces, because
//! something *did* happen: the user's history changed. That is why the status is
//! 3 and the result is [`Report::GoalRecorded`] rather than a [`NotYet`]. A run
//! that recorded a goal and checked nothing has to stop a script without being
//! silent about what it wrote.
//!
//! # Why the goal is bound to a project state
//!
//! The store's only project-aware write takes a fingerprint
//! ([`Store::append_for`](sure_core::store::Store::append_for)), and the binding
//! is right: a goal recorded with no project is a row no reader looking for *this
//! project's* goal can find. So SURE reads the project far enough to know which
//! state it is recording against, and reports the kind and the digest of that
//! state — the two fields that answer "which state was this?" for a later run.
//! The identifier the store is handed is minted per run and is deliberately not
//! what the report leads with; `docs/architecture/PROJECT_INTENT.md` records the
//! consequence for a reader.
//!
//! # Why the project is read before the store is opened
//!
//! Opening a store creates its directory and its file. If the project cannot be
//! read, SURE has nothing to record the goal against, and leaving a database
//! behind for a run that stored nothing would be a side effect of a failure.
//! Ordering the two this way costs nothing and means every refusal in this module
//! leaves the machine as it found it — which is a claim
//! `crates/sure-cli/tests/cli_contract.rs` can only make about the one refusal
//! that runs as a process.
//!
//! # Why a failure is status 5 and not 3
//!
//! 3 is "this build cannot carry that out", whose remedy is a newer build. This
//! command *can* carry out what it was asked. A run that met an unreadable
//! project or a store it could not open tried and did not finish, which is what 5
//! is for. One status for both is how a broken installation gets read as a build
//! that has nothing to do.
//!
//! # What this build still does not do
//!
//! It does not check the project, and it does not compare the goal against
//! anything. Every report it produces says so in the words a user reads, because
//! a goal that was recorded and then silently not used is exactly the shape of
//! failure this program exists to find in other people's work.

use std::path::{Path, PathBuf};

use sure_core::fingerprint::{FingerprintOptions, project_fingerprint};
use sure_core::intent::IntentSource;
use sure_core::paths::Paths;
use sure_core::project_intent::{EXPLICIT_GOAL_ID, explicit_goal, record};
use sure_core::store::Store;

use crate::cli::Command;
use crate::report::{Failed, GoalRecorded, NotYet, Report};

/// What `sure check` will do, once this build can.
const DOES: &str = "check a project and report what it found";

/// What a run that was not given a goal did instead.
///
/// The same sentence this command gave before it could store a goal at all. A
/// bare `sure check` still does nothing, and it still has to say so.
const NOTHING_CHECKED: &str = "Nothing was checked.";

/// What every failure in this module did instead.
///
/// One sentence for all of them, because it is true of all of them: each refusal
/// below happens before or instead of the write. The one exception says so in its
/// own words — see [`RECORDED_WITHOUT_A_ROW`].
const NOTHING_RECORDED: &str = "Nothing was recorded, and nothing was checked.";

/// What a run says when the goal was written and no row came back for it.
///
/// Separate from [`NOTHING_RECORDED`] because this is the one failure here in
/// which something *was* written. A report using the other sentence would tell
/// the user their history is unchanged when it is not, and they would have no way
/// to find out.
const RECORDED_WITHOUT_A_ROW: &str =
    "The goal was recorded, and SURE cannot say which record holds it.";

/// `sure check`, discovering where SURE keeps its files.
///
/// # Errors
///
/// None: a failure is a [`Report::Failed`], because a command that could not
/// finish still has to answer in the shape a caller reads.
#[must_use]
pub fn run(command: &Command, project: Option<&Path>, goal: Option<&str>) -> Report {
    // A bare `sure check` does nothing, so it does not look for anything either.
    // This is not only an optimisation: a machine where SURE cannot find its own
    // data directory must not turn a command that does nothing into a *different*
    // kind of nothing, and that is the branch discovery failing would take.
    if goal.is_none() {
        return not_yet(command);
    }
    let project = match project_of(project) {
        Ok(project) => project,
        Err(detail) => return failed(command, NOTHING_RECORDED, detail),
    };
    match Paths::discover() {
        Ok(paths) => run_with(command, &paths, &project, goal),
        // Not `Unavailable`: SURE has somewhere to keep its files or it does not,
        // and a machine where it does not is a machine to fix rather than a build
        // to update.
        Err(error) => failed(command, NOTHING_RECORDED, error.to_string()),
    }
}

/// The project this run is about: the one named, or the one SURE was run from.
///
/// The documented default is the current directory, and it is resolved to an
/// absolute path here rather than passed along as `.`. SURE refuses to
/// fingerprint a relative root, and the reason is worth keeping: `.` means
/// "wherever this process happens to be", so a record naming it can be read
/// later against a directory the user never meant. Refusing the absolute form of
/// that mistake is the whole job of the rule, and `.` is the same mistake spelled
/// shorter.
///
/// A path the user gave is passed through untouched, because SURE does not
/// silently repair an argument: `sure check not-a-project --goal "…"` has to say
/// that the root is relative, not quietly check somewhere else.
fn project_of(project: Option<&Path>) -> Result<PathBuf, String> {
    match project {
        Some(path) => Ok(path.to_path_buf()),
        None => std::env::current_dir().map_err(|error| {
            format!(
                "SURE could not work out which directory it is running in, so it could not \
                 tell which project this goal is about. Name the project explicitly to \
                 record the goal anyway: {error}"
            )
        }),
    }
}

/// The same, with the locations named rather than discovered.
///
/// Separate from [`run`] so that a test can point SURE at a store it made instead
/// of at the user's own. The alternative was not available: on Windows the
/// per-user data directory is read through `SHGetKnownFolderPath`, which ignores
/// `LOCALAPPDATA`, so there is no environment variable a test could set. A test
/// that ran the real path would be adding an invented requirement to the history
/// of the machine it ran on, and it would look exactly like a passing test.
#[must_use]
pub fn run_with(command: &Command, paths: &Paths, project: &Path, goal: Option<&str>) -> Report {
    let Some(goal) = goal else {
        return not_yet(command);
    };

    // The words first. A goal with no words in it is not a requirement, and
    // finding that out needs nothing opened, nothing read and nothing created.
    let intent = match explicit_goal(goal) {
        Ok(intent) => intent,
        Err(error) => return failed(command, NOTHING_RECORDED, error.to_string()),
    };

    // The project as the store takes it — text — before anything is read. A path
    // that cannot be written down is one SURE could not name in the record it was
    // about to write, and the store would have to lose it to proceed.
    let Some(root) = project.to_str() else {
        return failed(
            command,
            NOTHING_RECORDED,
            format!(
                "The project path {} is not text SURE can write down, so it could not name \
                 the project this goal is about.",
                project.display()
            ),
        );
    };

    // Which state of the project this is being recorded against, read before the
    // store is opened — see the module comment.
    let state = match project_fingerprint(project, &FingerprintOptions::default()) {
        Ok(state) => state,
        Err(error) => return failed(command, NOTHING_RECORDED, error.to_string()),
    };

    let store = match Store::open(paths, project) {
        Ok(store) => store,
        Err(error) => return failed(command, NOTHING_RECORDED, error.to_string()),
    };

    let rows = match record(&store, root, &state.id, &intent) {
        Ok(rows) => rows,
        Err(error) => return failed(command, NOTHING_RECORDED, error.to_string()),
    };

    // One row, because the intent this module builds has one requirement and the
    // schema stores one requirement per row. Written as a match rather than as
    // `rows[0]` so that a later version of `explicit_goal` producing several is a
    // refusal here rather than a report naming the first of them as *the* record
    // — and so that a store that returned nothing is not reported as record #0.
    match rows.first().copied() {
        Some(row) => Report::GoalRecorded(Box::new(GoalRecorded {
            goal: goal.to_owned(),
            requirement_id: EXPLICIT_GOAL_ID.to_owned(),
            source: IntentSource::ExplicitUserGoal,
            project_root: root.to_owned(),
            project_state: state,
            record: row,
        })),
        None => failed(
            command,
            RECORDED_WITHOUT_A_ROW,
            "The store accepted the goal and returned no row identifier for it. The goal is \
             in the history; SURE cannot say which record it is. This is a fault in SURE \
             rather than in the goal."
                .to_owned(),
        ),
    }
}

/// The refusal for a run that was not asked to record anything.
fn not_yet(command: &Command) -> Report {
    Report::Unavailable(NotYet {
        command: command.name(),
        does: DOES,
        instead: NOTHING_CHECKED,
    })
}

/// A run that tried and could not finish.
fn failed(command: &Command, what: &'static str, detail: String) -> Report {
    Report::Failed(Box::new(Failed {
        command: command.name(),
        what,
        detail,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use serde_json::json;
    use sure_core::store::{HistoryFilter, RecordKind, Store, StoredRecord};

    use super::*;
    use crate::report::exit;

    /// A directory under the workspace's git-ignored `target/tmp`, holding
    /// everything one test needs to be a different machine.
    ///
    /// Unique per call and never cleared, which is the pattern this repository
    /// settled on: clearing a fixed path and then treating it as fresh fails on
    /// Windows, and the test then describes a directory that was never emptied.
    /// Uniqueness comes from `create_dir`, not from the name, so two processes
    /// given the same id cannot collide.
    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(test: &str) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let base = sure_testkit::repository_root()
                .join("target")
                .join("tmp")
                .join("sure check");
            std::fs::create_dir_all(&base)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
            for _ in 0..1_000 {
                let root = base.join(format!("{test}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
                match std::fs::create_dir(&root) {
                    Ok(()) => {
                        std::fs::create_dir_all(root.join("project"))
                            .unwrap_or_else(|error| panic!("cannot create the project: {error}"));
                        return Self { root };
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("cannot create {}: {error}", root.display()),
                }
            }
            panic!("no free fixture name under {}", base.display());
        }

        /// The project `sure check` is pointed at.
        fn project(&self) -> PathBuf {
            self.root.join("project")
        }

        fn write(&self, relative: &str, contents: &str) {
            let full = self.project().join(relative);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
            }
            std::fs::write(&full, contents)
                .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        }

        /// Where SURE keeps its files for this test: nowhere near the project,
        /// because a store inside the tree being judged is refused.
        fn paths(&self) -> Paths {
            Paths::from_roots(self.root.join("data"), self.root.join("config"))
                .expect("the scratch locations are absolute")
        }
    }

    /// The command a user would have typed to get here.
    fn check() -> Command {
        Command::Check {
            path: None,
            goal: None,
        }
    }

    /// Everything the store holds, recordings included.
    fn everything(store: &Store) -> Vec<StoredRecord> {
        store
            .history(
                &HistoryFilter {
                    include_recordings: true,
                    ..HistoryFilter::default()
                },
                100,
            )
            .unwrap_or_else(|error| panic!("cannot read the history: {error}"))
    }

    /// The record kind a project intent is stored as.
    ///
    /// Asked of the protocol's own table rather than restated here, and reached
    /// through `RecordKind::from_name` because that is the door this crate's
    /// manifest gives the CLI into `sure_protocol::documents`: `sure-cli` has one
    /// edge into the engine and its vocabulary (ADR 0001), and a test needing to
    /// name a document kind is not a reason to open a second.
    fn intent_kind() -> RecordKind {
        // The wire name, which is not the Rust variant's spelling:
        // `project-intent`, with the hyphen, and the schema file follows it.
        RecordKind::from_name("project-intent").expect("the protocol defines this kind")
    }

    /// The one project-intent row in a store, or a panic naming what was there.
    fn the_intent_row(store: &Store) -> StoredRecord {
        let mut intents: Vec<StoredRecord> = store
            .history(&HistoryFilter::default(), 100)
            .unwrap_or_else(|error| panic!("cannot read the history: {error}"))
            .into_iter()
            .filter(|row| row.kind == intent_kind())
            .collect();
        assert_eq!(
            intents.len(),
            1,
            "expected exactly one project-intent row, found {}",
            intents.len()
        );
        intents.remove(0)
    }

    fn recorded(report: &Report) -> &GoalRecorded {
        match report {
            Report::GoalRecorded(recorded) => recorded,
            other => panic!("expected a recorded goal, got {other:?}"),
        }
    }

    fn failure(report: &Report) -> &Failed {
        match report {
            Report::Failed(failure) => failure,
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn a_goal_typed_to_sure_is_stored_and_read_back_with_the_project_it_is_about() {
        // The acceptance, end to end through the entry point the CLI calls. The
        // store is one this test made, so "read back" is a claim about a file
        // rather than about a value that never left the process.
        let fixture = Fixture::new("goal-stored");
        fixture.write("src/lib.rs", "pub fn answer() -> u32 { 42 }\n");
        let paths = fixture.paths();
        let project = fixture.project();

        let goal = "make the upload reject a file over 10 MB instead of failing silently";
        let report = run_with(&check(), &paths, &project, Some(goal));

        let recorded = recorded(&report);
        assert_eq!(recorded.goal, goal);
        assert_eq!(recorded.requirement_id, EXPLICIT_GOAL_ID);
        assert_eq!(recorded.source, IntentSource::ExplicitUserGoal);
        assert_eq!(recorded.project_root, project.to_str().unwrap());
        assert!(recorded.record > 0, "a row identifier is not a row number");
        assert!(
            !recorded.project_state.digest.is_empty(),
            "the report says which project state the goal is about and does not say which"
        );

        // Not a success. `sure check` in CI must not pass while checking nothing.
        assert_eq!(report.exit_code(), exit::UNAVAILABLE);
        assert_ne!(report.exit_code(), exit::OK);
        assert_eq!(report.outcome(), "unavailable");
        assert_eq!(report.command(), "check");

        let store = Store::open_at(&paths.store_file())
            .unwrap_or_else(|error| panic!("cannot open the store this run wrote: {error}"));
        let row = the_intent_row(&store);
        assert_eq!(row.id, recorded.record);
        assert_eq!(row.project_root.as_deref(), Some(project.to_str().unwrap()));
        assert_eq!(
            row.project_fingerprint.as_deref(),
            Some(recorded.project_state.id.as_str())
        );
        assert_eq!(row.document["text"], json!(goal));
        assert_eq!(row.document["source"], json!("explicit_user_goal"));
        assert!(row.document["raw_retained"].as_bool().unwrap_or(false));
    }

    #[test]
    fn storing_a_goal_writes_no_recording_and_the_goal_is_not_summarized() {
        // The two things a user cannot check for themselves. The first is the
        // acceptance's second half: nothing captured, nothing kept that was not
        // handed over in the same breath. The second is the same rule from the
        // other side — the words are the user's, so a run that trimmed or
        // rewrapped them would be storing a requirement nobody stated.
        let fixture = Fixture::new("goal-privacy");
        fixture.write("README.md", "# a project\n");
        let paths = fixture.paths();

        let goal = "  keep the two-space indent\nand do not trim the trailing tab\t";
        let report = run_with(&check(), &paths, &fixture.project(), Some(goal));
        assert_eq!(
            recorded(&report).goal,
            goal,
            "the goal was edited on the way in"
        );

        let store = Store::open_at(&paths.store_file()).expect("the store this run wrote");
        assert!(
            store
                .history(&HistoryFilter::recordings(), 100)
                .expect("the recordings")
                .is_empty(),
            "recording a goal produced a raw recording"
        );
        for row in everything(&store) {
            assert!(
                !row.kind.is_recording(),
                "recording a goal wrote a recording: {}",
                row.document
            );
        }
        assert_eq!(the_intent_row(&store).document["text"], json!(goal));
    }

    #[test]
    fn the_same_project_state_is_the_same_state_twice_and_a_changed_one_is_not() {
        // What makes the fingerprint in the record an anchor rather than a token
        // that happens to be written down. It is read from two runs, so it is not
        // a claim about the fingerprinter — there are tests for that — but about
        // what this module records: the state of the project as it was when the
        // user said what they wanted.
        let fixture = Fixture::new("goal-fingerprint");
        fixture.write("main.py", "print('one')\n");
        let paths = fixture.paths();
        let project = fixture.project();

        let first = run_with(&check(), &paths, &project, Some("first thing"));
        let again = run_with(&check(), &paths, &project, Some("second thing"));
        assert_eq!(
            recorded(&first).project_state.digest,
            recorded(&again).project_state.digest,
            "an unchanged project fingerprinted differently on a second run, so the \
             recorded state says nothing about the project"
        );

        fixture.write("main.py", "print('two')\n");
        let changed = run_with(&check(), &paths, &project, Some("third thing"));
        assert_ne!(
            recorded(&first).project_state.digest,
            recorded(&changed).project_state.digest,
            "a changed project fingerprinted the same, so a goal could be read against a \
             state it was never stated for"
        );
    }

    #[test]
    fn a_goal_with_no_words_is_refused_before_the_store_exists() {
        // The refusal is in the value, so a caller that gets it never opened a
        // database to fail at it — and a user who typed nothing has no record to
        // clean up. The store's absence is the assertion that says so.
        let fixture = Fixture::new("goal-empty");
        let paths = fixture.paths();

        for goal in ["", "   ", "\t\n"] {
            let report = run_with(&check(), &paths, &fixture.project(), Some(goal));
            let failure = failure(&report);
            assert_eq!(failure.command, "check");
            assert_eq!(failure.what, NOTHING_RECORDED);
            assert!(
                !failure.detail.is_empty(),
                "the failure does not say what stopped it"
            );
            assert_eq!(report.exit_code(), exit::FAILED);
            assert_ne!(report.exit_code(), exit::UNAVAILABLE);
            assert!(
                !paths.store_file().exists(),
                "{goal:?} created the store before deciding it was not a goal"
            );
        }
    }

    #[test]
    fn a_project_that_cannot_be_read_leaves_no_store_behind() {
        // The order of the two steps, as a fact about the filesystem. Opening a
        // store creates its directory and its file, so if the fingerprint were
        // taken afterwards, a run that recorded nothing would still have left a
        // database where the user keeps their history.
        let fixture = Fixture::new("goal-unreadable");
        let paths = fixture.paths();

        // Two projects that cannot be read, and they fail for different reasons:
        // which is the point, because a single case would leave the ordering
        // resting on one error path.
        //
        // The first is relative, which cannot succeed on any machine: SURE
        // refuses to resolve one against the current directory, because for a
        // check the current directory is the project.
        //
        // The second is absolute and outside the store, and does not exist —
        // the ordinary mistake of a mistyped path. It matters that this one is
        // here: a relative root is refused by `Store::open`'s own boundary check
        // as well, so a version of this module that opened the store *first* and
        // read the project afterwards would still pass the relative case. The
        // missing directory is refused by the fingerprinter and by nothing else,
        // and a store opened before it would be a store left behind.
        let missing = fixture.root.join("not-a-directory");
        for project in [Path::new("not-a-project"), missing.as_path()] {
            let report = run_with(&check(), &paths, project, Some("do it"));
            let failure = failure(&report);
            assert_eq!(failure.what, NOTHING_RECORDED, "{}", project.display());
            assert_eq!(report.exit_code(), exit::FAILED);
            assert!(
                failure.detail.len() > 20,
                "{} reports {failure:?}, which does not say what stopped it",
                project.display()
            );
            assert!(
                !paths.store_file().exists(),
                "a project SURE could not read ({}) left a store behind",
                project.display()
            );
            assert!(
                !paths.data_dir().exists(),
                "a project SURE could not read ({}) left its data directory behind",
                project.display()
            );
        }
    }

    #[test]
    fn the_store_may_not_be_inside_the_project_it_records_a_goal_for() {
        // `docs/architecture/STORAGE_AND_DATA_PATHS.md`: a project the agent can
        // edit is not a trusted place for evidence about that project, and a
        // history the project can rewrite cannot support a verdict. The rule is
        // `Paths::ensure_outside`'s and this test is only that the check goes
        // through it rather than around it.
        let fixture = Fixture::new("goal-inside");
        fixture.write("notes.txt", "hello\n");
        let project = fixture.project();
        let inside = Paths::from_roots(project.join(".sure"), fixture.root.join("config"))
            .expect("the scratch locations are absolute");

        let report = run_with(&check(), &inside, &project, Some("do the thing"));
        assert_eq!(failure(&report).what, NOTHING_RECORDED);
        assert_eq!(report.exit_code(), exit::FAILED);
        assert!(
            !inside.store_file().exists(),
            "SURE put its own evidence inside the project it was judging"
        );
    }

    #[test]
    fn check_without_a_goal_is_the_same_refusal_it_has_always_been() {
        // The half of this command that did not change, held still. A bare
        // `sure check` does nothing, so it must say so — and it must not look for
        // its files while saying it, because a machine where it cannot find them
        // has to give the same answer as one where it can.
        let fixture = Fixture::new("goal-absent");
        let paths = fixture.paths();
        let project = fixture.project();

        let report = run_with(&check(), &paths, &project, None);
        let Report::Unavailable(not_yet) = &report else {
            panic!("a bare `sure check` answered {report:?}");
        };
        assert_eq!(not_yet.command, "check");
        assert_eq!(not_yet.does, DOES);
        assert_eq!(not_yet.instead, NOTHING_CHECKED);
        // Which of the two things did not happen, in the words the user reads.
        // The constant is named above and the sentence is asserted here, because
        // the two are different claims: a bare `sure check` recorded nothing
        // *and* checked nothing, and a refusal that said only the first would
        // leave a user wondering whether the check happened. The `--goal` path
        // has the other sentence for the other reason — see `NOTHING_RECORDED`.
        assert!(
            not_yet.instead.contains("checked"),
            "a bare `sure check` does not say the check did not happen: {:?}",
            not_yet.instead
        );
        assert!(
            !not_yet.instead.contains("recorded"),
            "a bare `sure check` answers about the history rather than about the \
             check: {:?}",
            not_yet.instead
        );
        assert_eq!(report.exit_code(), exit::UNAVAILABLE);
        assert!(
            !paths.store_file().exists(),
            "a check with nothing to do created a store"
        );

        // The same report, whether the locations are discovered or given. The
        // discovered one is not asserted to succeed on every machine, only to
        // agree: neither path may depend on where SURE keeps its files.
        assert_eq!(run(&check(), Some(&project), None), report);
    }

    #[test]
    fn a_check_with_no_path_is_about_the_directory_sure_was_run_from() {
        // The documented default, resolved rather than handed on as `.`. A run
        // that passed `.` through would be refused by the fingerprinter for being
        // relative — a refusal about the spelling of an argument the user never
        // typed, since the path was not given at all.
        let resolved = project_of(None).expect("a test process has a current directory");
        assert!(
            resolved.is_absolute(),
            "the default project is not an absolute path: {resolved:?}"
        );
        assert_eq!(resolved, std::env::current_dir().unwrap());

        // A path that *was* given is not touched: SURE refuses a relative root
        // rather than resolving it against wherever it happens to be, and a
        // caller that repaired the argument instead would be checking a project
        // the user did not name.
        assert_eq!(
            project_of(Some(Path::new("not-a-project"))).unwrap(),
            PathBuf::from("not-a-project")
        );
    }

    #[test]
    fn every_refusal_in_this_module_says_what_did_not_happen() {
        // `crate::report::Failed`'s own rule: the fixed sentence is what tells a
        // user whether their history changed. Asserted over every failure this
        // module can produce, because the one that got it wrong would be the one
        // nobody drove.
        let fixture = Fixture::new("goal-failure-text");
        let fixture_paths = fixture.paths();
        let project = fixture.project();

        let cases: Vec<(&str, Report)> = vec![
            (
                "a goal with no words",
                run_with(&check(), &fixture_paths, &project, Some("   ")),
            ),
            (
                "a project path that is not text",
                run_with(
                    &check(),
                    &fixture_paths,
                    Path::new("relative"),
                    Some("do it"),
                ),
            ),
            (
                "a store inside the project",
                run_with(
                    &check(),
                    &Paths::from_roots(project.join(".sure"), fixture.root.join("config"))
                        .expect("absolute"),
                    &project,
                    Some("do it"),
                ),
            ),
        ];

        for (what_happened, report) in cases {
            let failure = failure(&report);
            assert_eq!(failure.command, "check", "{what_happened}");
            assert_eq!(
                failure.what, NOTHING_RECORDED,
                "{what_happened} does not say whether the history changed"
            );
            assert!(
                failure.detail.len() > 20,
                "{what_happened} reports {failure:?}, which does not tell the user what \
                 stopped it"
            );
            // The word a user needs in order to believe nothing changed.
            assert!(
                failure.what.contains("Nothing was recorded"),
                "{what_happened}: {}",
                failure.what
            );
        }
    }
}

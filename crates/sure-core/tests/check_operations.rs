//! `P18-T003`'s acceptance, checked from outside the crate.
//!
//! The task's sentence has four clauses and two of them are this file's:
//!
//! > *Every check SURE already proposes for the three supported ecosystems has
//! > typed work behind it.*
//! >
//! > *The Windows package-manager name is covered by a test on the resolution
//! > itself.*
//!
//! # Why this file exists beside the three module tests
//!
//! Each of `checks/node.rs`, `checks/python.rs` and `checks/rust.rs` now holds its
//! own `the_work_behind_a_check_is_the_typed_form_of_the_line_a_report_prints`,
//! and between them they sweep the whole manager-by-role matrix. That is the right
//! place for a matrix. What none of them can hold is the claim this file is about:
//! that **manifests a person would write**, read by the product's own discovery
//! off a disk, reach a plan in which every check has work behind it. All three
//! module tests could pass while the discovery-to-check path lost the operation on
//! the way, because each of them builds its project value by hand.
//!
//! So every fixture below is files on disk under the workspace's own `target/`,
//! with a space and a non-ASCII character in the path, and every assertion is made
//! through the public API. The two rules this file adds to the matrix are the two
//! the matrix cannot see:
//!
//! 1. **The line a report prints is the typed vector, for every check of all three
//!    ecosystems at once.** Rendering a `CommandSpec` back into a string and
//!    comparing it with `CheckReason::DeclaredCommand`'s `command` is the direction
//!    that fails when either half moves alone — the rendering is what reports and
//!    wire formats keep, and the vector is what a runner would be handed.
//! 2. **A member's check runs in the member's directory.** The same program in two
//!    directories is two different facts about a project, and a member's directory
//!    appears nowhere in the command a report prints.
//!
//! # The Windows half, and why it is a test rather than a sentence
//!
//! A bare `npm` is not a program `CreateProcess` will start: it completes a name
//! with no extension with `.exe` and nothing else, so a machine whose only npm is
//! the `.cmd` one Node ships has an `npm` that resolves to a batch file. SURE's
//! builders must **not** paper over that — appending `.cmd`, or wrapping the name
//! in `cmd.exe /c`, would be constructing the interpreter ADR 0014 refuses — and
//! the thing that answers the question honestly is
//! [`ProgramPath`](sure_core::planned_work::ProgramPath).
//!
//! `the_program_a_node_check_plans_is_resolved_against_a_machine_that_has_only_the_script`
//! takes the program **out of a check the product built** and resolves it against
//! a `PATH` this test writes, so what is resolved is the builder's own name and
//! not a literal in a test. The machine is a directory in this process's scratch
//! space: a resolution test that read the developer's real `PATH` would be a test
//! about the machine it runs on.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::checks::node::NodeChecks;
use sure_core::checks::python::PythonChecks;
use sure_core::checks::rust::RustChecks;
use sure_core::discover::node::MANIFEST as NODE_MANIFEST;
use sure_core::discover::rust::MANIFEST as RUST_MANIFEST;
use sure_core::discover::{DiscoverOptions, Discovery, Ecosystem, Findings, discover};
use sure_core::planned_work::{CheckOperation, PlannedWork};
use sure_core::schedule::CheckReason;

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use sure_core::discover::node::PackageManager;
#[cfg(windows)]
use sure_core::planned_work::{ProgramPath, Resolution};

/// A scratch project that removes itself.
///
/// Under the workspace's own `target/` so the fixture is on the same volume as
/// the checkout, with a space and a non-ASCII character in the path — the
/// cheapest way to make every path below one two platforms disagree about, which
/// is the discipline `CLAUDE.md` asks for and which `discover_node.rs`
/// established.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let project = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure 指纹 operations")
            .join(unique);
        std::fs::create_dir_all(&project)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", project.display()));
        Self { project }
    }

    fn path(&self) -> &Path {
        &self.project
    }

    /// Write a file inside the project, creating the directories above it.
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

    fn discover(&self) -> sure_core::discover::Discovery {
        discover(&self.project, &DiscoverOptions::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// The Node findings, or a panic naming what was found instead.
    ///
    /// Cloned rather than borrowed because the discovery it comes from is a
    /// temporary here, and `Drop` on the discovery would free it. Cloning is the
    /// cheap half of this file: what it costs is a `NodeProject` per call, and what
    /// it buys is that a caller cannot hold a finding that outlives the reading it
    /// was made from.
    fn node(&self) -> sure_core::discover::NodeProject {
        let found = self.discover();
        match finding(&found, Ecosystem::Node, self.path()) {
            Findings::Node(node) => (**node).clone(),
            other => panic!("the Node report carried {other:?}"),
        }
    }

    /// Every check the three ecosystems' own builders propose for this project,
    /// with the ecosystem each one came from.
    ///
    /// One discovery for the three, because that is what a run does: a project
    /// that is a Node and a Python and a Rust project at once is one directory
    /// read once, and a builder handed a second reading could plan a check about a
    /// file that had changed in between.
    fn ecosystem_checks(&self) -> Vec<(&'static str, PlannedWork)> {
        let found = self.discover();
        let mut planned: Vec<(&'static str, PlannedWork)> = Vec::new();

        match finding(&found, Ecosystem::Node, self.path()) {
            Findings::Node(node) => planned.extend(
                NodeChecks::of(node, self.path())
                    .planned()
                    .iter()
                    .cloned()
                    .map(|work| ("node", work)),
            ),
            other => panic!("the Node report carried {other:?}"),
        }

        match finding(&found, Ecosystem::Python, self.path()) {
            Findings::Python(python) => planned.extend(
                PythonChecks::of(python, self.path())
                    .planned()
                    .iter()
                    .cloned()
                    .map(|work| ("python", work)),
            ),
            other => panic!("the Python report carried {other:?}"),
        }

        match finding(&found, Ecosystem::Rust, self.path()) {
            Findings::Rust(rust) => planned.extend(
                RustChecks::of(rust, self.path())
                    .planned()
                    .iter()
                    .cloned()
                    .map(|work| ("rust", work)),
            ),
            other => panic!("the Rust report carried {other:?}"),
        }

        planned
    }
}

/// One ecosystem's findings from a discovery, or a panic naming what was found
/// instead.
///
/// Never `None`: a fixture that quietly stopped being three projects would make
/// every assertion in the tests below vacuous, and a vacuous pass is the false
/// green this repository ranks above a visible error.
fn finding<'a>(found: &'a Discovery, ecosystem: Ecosystem, root: &Path) -> &'a Findings {
    let report = found.report(ecosystem).unwrap_or_else(|| {
        panic!(
            "{} declares a {} project and discovery did not report one; it looked for {:?}",
            root.display(),
            ecosystem.as_str(),
            found
                .looked_for()
                .iter()
                .map(|ecosystem| ecosystem.as_str())
                .collect::<Vec<_>>()
        )
    });
    &report.findings
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        drop(std::fs::remove_dir_all(&self.project));
    }
}

/// A project a person would write, in all three ecosystems at once.
///
/// Written out rather than generated because each line below is a decision the
/// assertions rest on: the Node root has a `package-lock.json` so the manager can
/// only be npm and a member so there is a second directory, the Python manifest
/// declares all four tools and a `uv.lock` so its installer is uv, and the Rust
/// half ships both tool configuration files so all four of its checks exist.
fn three_ecosystems() -> Fixture {
    let fixture = Fixture::new("three-ecosystems");
    fixture
        .write(
            NODE_MANIFEST,
            r#"{"name":"root","workspaces":["packages/*"],"scripts":{
                 "build":"tsc -b","test":"vitest run","lint":"eslint .","typecheck":"tsc --noEmit"}}"#,
        )
        .write("package-lock.json", "{}\n")
        .write(
            "packages/web/package.json",
            r#"{"name":"web","scripts":{"build":"vite build"}}"#,
        )
        .write(
            "pyproject.toml",
            r#"
[project]
name = "demo"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = ["pytest>=8", "ruff>=0.4", "mypy>=1.8", "setuptools>=68"]

[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"
"#,
        )
        .write("uv.lock", "")
        .write(
            RUST_MANIFEST,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .write("Cargo.lock", "")
        .write("src/main.rs", "fn main() {\n    println!(\"demo\");\n}\n")
        .write("clippy.toml", "")
        .write("rustfmt.toml", "");
    fixture
}

/// The line a `CommandSpec` renders as: the program, a space, the arguments.
///
/// Joined here rather than read from a helper, because the point of the assertion
/// it feeds is that this rendering and the one a report prints are the same
/// string. A shared helper would let both drift together.
fn rendered(work: &PlannedWork) -> String {
    let CheckOperation::Command(spec) = work.operation() else {
        panic!("{:?} is planned without a command", work.proposal().title());
    };
    std::iter::once(spec.program().to_string_lossy().into_owned())
        .chain(
            spec.arguments()
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned()),
        )
        .collect::<Vec<String>>()
        .join(" ")
}

/// The declared line this check's reason carries, or a panic saying what it
/// carried instead.
fn declared_line(work: &PlannedWork) -> &str {
    match work.proposal().reason() {
        CheckReason::DeclaredCommand { command, .. } => command,
        other => panic!(
            "`{}` is a declared command and its reason is {other:?}",
            work.proposal().title()
        ),
    }
}

#[test]
fn every_check_the_three_ecosystems_propose_has_the_work_that_would_carry_it_out() {
    // The first acceptance clause, over **one project read once**: three real
    // manifests, the product's own discovery, and the three builders a run calls.
    // Every check that comes out of that path is asked the same four questions,
    // and the counts below are the guard that the loop is not empty.
    let fixture = three_ecosystems();
    let planned = fixture.ecosystem_checks();

    let mut counts: Vec<(&str, usize)> = Vec::new();
    for (ecosystem, work) in &planned {
        let ecosystem = *ecosystem;
        match counts.iter_mut().find(|(named, _)| *named == ecosystem) {
            Some((_, count)) => *count += 1,
            None => counts.push((ecosystem, 1)),
        }

        let title = work.proposal().title().to_owned();
        let CheckOperation::Command(spec) = work.operation() else {
            panic!(
                "{ecosystem}'s `{title}` is planned with {:?}. Every check the three \
                 ecosystems propose has the work that would carry it out behind it, so a \
                 runner handed this plan has something to run and a report has something to \
                 explain; a check with no operation is one of them silently missing",
                work.operation()
            );
        };

        // The operation is a command and a command is a process. Both are asked
        // because they are two different answers: `starts_a_process` is what a
        // runner asks before every entry, and the variant is what a report asks to
        // write the entry's own description.
        assert!(
            work.operation().starts_a_process(),
            "{ecosystem}'s `{title}` does not start a process"
        );
        assert!(!work.operation().plain_description().trim().is_empty());

        // The program is a name, and a name with no extension is not a program on
        // Windows — so a builder that appended `.cmd` here to make the plan look
        // runnable would be inventing the program it then starts. Whether the name
        // is startable is `ProgramPath`'s answer, taken by the runner.
        let program = spec.program().to_string_lossy().into_owned();
        assert!(
            !program.is_empty(),
            "{ecosystem}'s `{title}` names no program"
        );
        assert!(
            !program.contains('.'),
            "{ecosystem}'s `{title}` plans `{program}`, which carries an extension: no \
             builder in this tree appends one, and a name spelled with one is a different \
             question from the name a project declared"
        );

        assert!(
            !spec.arguments().is_empty(),
            "{ecosystem}'s `{title}` plans `{program}` with nothing after it, so the \
             command a person is asked to allow is not the command that would run"
        );

        // The directory is where the command runs, and it is the fact a rendered
        // line cannot carry. Absolute, because a relative one would mean "wherever
        // SURE happened to be started", and inside the project, because a check
        // that ran outside it would be about another directory.
        assert!(
            spec.working_directory().is_absolute(),
            "{ecosystem}'s `{title}` would run in {}, which is not absolute",
            spec.working_directory().display()
        );
        assert!(
            spec.working_directory().starts_with(fixture.path()),
            "{ecosystem}'s `{title}` runs in {}, which is outside the project",
            spec.working_directory().display()
        );

        // And the half that ties the operation to the report: the string a person
        // reads and the vector a runner would be handed are one decision shown
        // twice, so they are compared in the direction that fails when either
        // moves alone.
        assert_eq!(
            rendered(work),
            declared_line(work),
            "{ecosystem}'s `{title}` prints one command and would run another"
        );
    }

    counts.sort_unstable();
    assert_eq!(
        counts,
        vec![("node", 5), ("python", 4), ("rust", 4)],
        "the three ecosystems' builders proposed a different set of checks from the one \
         this fixture is about: the root's four roles and its member's one build, Python's \
         four roles, and Rust's four. A different set means the fixture stopped describing \
         what the assertions above are about rather than that the rule failed"
    );
}

#[test]
fn a_members_check_runs_in_the_members_directory_and_the_roots_in_the_root() {
    // **The fact ADR 0014 says a parsed display string cannot carry.** `npm run
    // build` in `packages/web` and `npm run build` at the root print the same line
    // and are two different commands. Over a real workspace this time, so the
    // directory is the one the discovery resolved from a `workspaces` pattern
    // rather than one a test built by hand.
    let fixture = three_ecosystems();
    let planned = fixture.ecosystem_checks();

    let mut directories = Vec::new();
    for (title, expected) in [
        ("build the project", fixture.path().to_path_buf()),
        (
            "build the project in packages/web",
            fixture.path().join("packages").join("web"),
        ),
    ] {
        let work = planned
            .iter()
            .find(|(_, work)| work.proposal().title() == title)
            .map(|(_, work)| work)
            .unwrap_or_else(|| panic!("no check titled `{title}` was proposed"));
        let CheckOperation::Command(spec) = work.operation() else {
            panic!("`{title}` is not a command");
        };
        assert_eq!(
            spec.working_directory(),
            expected,
            "`{title}` would run in the wrong directory"
        );
        directories.push(spec.working_directory().to_path_buf());
    }
    assert_ne!(
        directories[0], directories[1],
        "the root's check and the member's run in the same directory, so a member's \
         command would be run against the root's manifest"
    );
}

/// A directory a search path can point at, holding one file.
///
/// The `ProgramPath` fixtures below are machines rather than projects: what they
/// answer is *what is this name, on this path*, so they are built here rather than
/// written into the project above. `sure_testkit::scratch::directory` rather than
/// the fixture type, because these have no manifest and no discovery.
#[cfg(windows)]
fn a_machine_holding(name: &str, file: &str) -> PathBuf {
    let directory = sure_testkit::scratch::directory("sure check operations", name);
    std::fs::write(directory.join(file), b"fixture\n")
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", directory.join(file).display()));
    directory
}

/// The program a Node check plans, taken out of the product's own builder.
///
/// The name is read off a `CommandSpec` rather than written here, which is the
/// whole point of the two tests below: what is resolved is the name SURE would
/// plan, so a builder that started appending an extension would resolve to a
/// different answer than the one asserted.
#[cfg(windows)]
fn the_program_a_node_check_plans(lockfile: &str, manager: PackageManager) -> std::ffi::OsString {
    let fixture = Fixture::new(&format!("program-{}", manager.as_str()));
    fixture
        .write(
            NODE_MANIFEST,
            r#"{"name":"demo","scripts":{"build":"tsc -b"}}"#,
        )
        .write(lockfile, "");
    assert_eq!(
        fixture.node().managers.agreed(),
        Some(manager),
        "a project with {lockfile} and nothing else does not lock {manager:?}, so the name \
         resolved below would not be the manager's"
    );

    let checks = NodeChecks::of(&fixture.node(), fixture.path());
    let build = checks
        .planned()
        .iter()
        .find(|work| work.proposal().title() == "build the project")
        .unwrap_or_else(|| panic!("no build check for a project with a build script"));
    let CheckOperation::Command(spec) = build.operation() else {
        panic!("the build check does not run a command");
    };
    spec.program().to_owned()
}

/// The four managers a Node project can lock, and the file that locks each.
#[cfg(windows)]
const LOCKS: &[(&str, PackageManager)] = &[
    ("package-lock.json", PackageManager::Npm),
    ("yarn.lock", PackageManager::Yarn),
    ("pnpm-lock.yaml", PackageManager::Pnpm),
    ("bun.lockb", PackageManager::Bun),
];

/// The package-manager name, resolved on a machine whose only one is the script
/// Node installs beside it.
///
/// **The fourth acceptance clause, and the reason it is worth a test.** `npm` is
/// not a program on Windows: `CreateProcess` completes a name with no extension
/// with `.exe` and nothing else, so the `npm` a Node installation puts on `PATH`
/// — `npm.cmd` — is a file this build will not start. The honest answer is
/// `InterpreterRequired`, which a report renders as *there is an `npm.cmd` here
/// and this build will not start it*, and the dishonest one is a plan that says
/// nothing is installed. Neither is a spawn failure: whatever happens next is
/// P18-T007's, and what this holds is that the name the builder planned is the
/// name the resolution was asked about, and that nothing rewrote it on the way.
#[cfg(windows)]
#[test]
fn the_program_a_node_check_plans_is_resolved_against_a_machine_that_has_only_the_script() {
    for &(lockfile, manager) in LOCKS {
        let program = the_program_a_node_check_plans(lockfile, manager);
        let name = program.to_string_lossy().into_owned();
        assert_eq!(
            name,
            manager.as_str(),
            "the program planned for a {manager:?} project is not the manager's own name"
        );

        let machine = a_machine_holding(&format!("script-{name}"), &format!("{name}.cmd"));
        let script = machine.join(format!("{name}.cmd"));
        let path = ProgramPath::from_search_path(machine.clone().into_os_string());
        let resolved = path.resolve(&program);

        assert_eq!(
            resolved,
            Resolution::InterpreterRequired(script.clone()),
            "a machine whose only `{name}` is `{name}.cmd` must be told that is a script and \
             not that nothing is installed, so that a report can say which of the two it is"
        );
        assert_eq!(
            resolved.program_name(),
            Some(script.as_path()),
            "the file the name resolved to is not the file that is there"
        );
        assert!(
            !resolved.is_startable(),
            "`{name}.cmd` is a batch file and this build does not start one: reporting it as \
             startable would be a plan a runner would fail at"
        );
        assert!(!resolved.plain_description().trim().is_empty());
    }
}

/// And the same name, on a machine that has the executable.
///
/// The other half of the resolution, and the half that keeps the test above from
/// being satisfied by an implementation that answered "script" to everything: the
/// same name, asked of a `PATH` that holds `<name>.exe`, is a program this build
/// starts. Written in the same file so that the two answers are one comparison
/// apart.
#[cfg(windows)]
#[test]
fn the_program_a_node_check_plans_is_startable_on_a_machine_that_has_the_executable() {
    for &(lockfile, manager) in LOCKS {
        let program = the_program_a_node_check_plans(lockfile, manager);
        let name = program.to_string_lossy().into_owned();

        let machine = a_machine_holding(&format!("executable-{name}"), &format!("{name}.exe"));
        let executable = machine.join(format!("{name}.exe"));
        let path = ProgramPath::from_search_path(machine.into_os_string());
        let resolved = path.resolve(&program);

        assert_eq!(
            resolved,
            Resolution::Executable(executable),
            "`{name}` is on this path as an image this build starts, and that is what must \
             come back"
        );
        assert!(
            resolved.is_startable(),
            "an `.exe` on the search path is a program this build starts"
        );
    }
}

/// The name is not completed by hand, stated as the absence it is.
///
/// This is the rule the two tests above rest on, said in the one place a reader
/// would look for it: a search path holding only `<name>.cmd` and no `<name>.exe`
/// means the bare name is **not** a program, which is what makes
/// `Resolution::Executable` above a claim about the file that is there rather than
/// about the string. A machine with neither file answers `Absent`, and the three
/// answers are three different sentences.
#[cfg(windows)]
#[test]
fn a_machine_with_neither_file_answers_absent_and_not_an_error() {
    for &(_, manager) in LOCKS {
        let name = manager.as_str();
        let machine = a_machine_holding(&format!("absent-{name}"), "something-else.exe");
        let path = ProgramPath::from_search_path(machine.into_os_string());
        assert_eq!(
            path.resolve(OsStr::new(name)),
            Resolution::Absent,
            "only `something-else.exe` is there, so `{name}` is absent"
        );
    }
}

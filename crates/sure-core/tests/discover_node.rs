//! Finding out what a JavaScript or TypeScript project declares.
//!
//! P2-T004 acceptance:
//!
//! > Detects package managers, workspace structure, declared scripts and common
//! > web/server frameworks.
//!
//! `docs/architecture/ECOSYSTEM_DISCOVERY.md` is the authority for what is
//! asserted here, and its "Enforced by" table is the index from a statement to
//! the test below that holds it.
//!
//! # Why these tests build real projects
//!
//! Every criterion is about reading files a person wrote, and the parts that can
//! be unit-tested — the parsers, the tables, the rules — already are. What is
//! left, and what only a real directory can answer, is whether discovery finds
//! the files at all: whether the walk it reads from lists them, whether a
//! pattern resolves against the directories that are actually there, and whether
//! a file it could not read is reported instead of being treated as a file that
//! is not there.
//!
//! # The two properties most of this file is about
//!
//! **A manifest that could not be read is not a manifest that is absent.** A
//! project whose `package.json` failed to parse declares no scripts, no
//! dependencies and no package manager — and so does a project with no
//! `package.json` at all. Reporting the second when the first is true is a false
//! statement about the project, and it is the failure `CLAUDE.md` ranks above a
//! visible error. Several tests below exist only to hold that line.
//!
//! **Discovery at this stage runs nothing.** Reading a manifest is not executing
//! a project. One test writes a script whose command would leave a file behind
//! if anything ran it, and checks that no such file appears.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::discover::node::{
    DependencyKind, Disagreement, ManifestState, MemberManifest, PackageManager, ScriptRole,
    ToolRole, TypeScriptEvidence, UnresolvedReason,
};
use sure_core::discover::{
    DiscoverOptions, Discovery, Ecosystem, Findings, UnreadReason, discover,
};
use sure_core::scan::{ScanOptions, SkipReason};
use sure_core::vocabulary::SupportLevel;

/// A scratch project that removes itself.
///
/// Under the workspace's own `target/` so that the fixture is on the same volume
/// as the checkout, and with a space and a non-ASCII character in the path —
/// which is the cheapest way to make every path below one that two platforms
/// disagree about.
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
            .join("sure 指纹 discover")
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

    /// Write a directory, with nothing in it.
    fn mkdir(&self, relative: &str) -> &Self {
        let full = self.project.join(relative);
        std::fs::create_dir_all(&full)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", full.display()));
        self
    }

    fn discover(&self) -> Discovery {
        self.discover_with(DiscoverOptions::default())
    }

    fn discover_with(&self, options: DiscoverOptions) -> Discovery {
        discover(&self.project, &options).unwrap_or_else(|error| panic!("{error}"))
    }

    /// The Node findings, or a panic naming what was found instead.
    fn node(&self) -> sure_core::discover::NodeProject {
        self.node_with(DiscoverOptions::default())
    }

    fn node_with(&self, options: DiscoverOptions) -> sure_core::discover::NodeProject {
        let found = self.discover_with(options);
        let report = found.report(Ecosystem::Node).unwrap_or_else(|| {
            panic!(
                "this fixture is a Node project and discovery did not report one; \
                 it looked for {:?}",
                found
                    .looked_for()
                    .iter()
                    .map(|ecosystem| ecosystem.as_str())
                    .collect::<Vec<_>>()
            )
        });
        match &report.findings {
            Findings::Node(node) => (**node).clone(),
        }
    }

    /// Whether a path inside the project is there, asked of the filesystem.
    ///
    /// **The premise asserted with the tool that is not under test.** A test
    /// that says "discovery found the member at `packages/app`" is worth nothing
    /// unless `packages/app` is really there; asking discovery whether it is
    /// there would be asking the defendant. This asks `std::fs`.
    fn exists(&self, relative: &str) -> bool {
        self.project.join(relative).exists()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        // Windows keeps directory handles open longer than Unix does, so a
        // reader may still hold one. `remove_dir_all` does not follow a link —
        // the same property `scan_project.rs` relies on for its junction
        // fixtures — so a link out of the project does not take its target with
        // it.
        let _ = std::fs::remove_dir_all(&self.project);
    }
}

/// Create a directory link at `link` pointing at `target`.
///
/// Two platforms, two mechanisms, one rule. A junction rather than a symbolic
/// link on Windows: `mklink /J` needs no privilege, and Developer Mode and
/// administrator rights were both probed on this host and are both absent. The
/// arguments must be spelled with backslashes, because `mklink` reads `/` as the
/// start of one of its own switches.
#[cfg(windows)]
fn link_to_directory(target: &Path, link: &Path) {
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap_or_else(|error| panic!("cannot run cmd: {error}"));
    assert!(
        output.status.success(),
        "mklink /J {} {} failed: {}{}",
        link.display(),
        target.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn link_to_directory(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap_or_else(|error| {
        panic!(
            "cannot link {} to {}: {error}",
            link.display(),
            target.display()
        )
    });
}

/// The script behind a conventional role, or `None` if the project declared none.
///
/// Panics when the role has no *row*, which is a different thing from a row with
/// no script: the row is what says the project does not have one, and a missing
/// row would mean the report had nothing to say at all.
fn script_named(
    rows: &[sure_core::discover::node::ConventionalScript],
    role: ScriptRole,
) -> Option<&sure_core::discover::node::Script> {
    rows.iter()
        .find(|row| row.role == role)
        .unwrap_or_else(|| panic!("{role:?} has no row"))
        .script
        .as_ref()
}

// ---------------------------------------------------------------------------
// The project has to be one at all
// ---------------------------------------------------------------------------

#[test]
fn a_directory_with_no_node_files_in_it_is_not_a_node_project() {
    let fixture = Fixture::new("not-node");
    fixture.write("README.md", "# nothing to see");

    let found = fixture.discover();
    assert!(
        found.report(Ecosystem::Node).is_none(),
        "a directory with a README is not a Node project"
    );
    // The part that matters: `ecosystems` being empty means *none of these were
    // found*, and the list of what was looked for is what makes that a
    // statement instead of a shrug.
    assert_eq!(
        found.looked_for(),
        &[Ecosystem::Node],
        "an empty result must still say what was looked for"
    );
    assert!(found.is_complete());
}

#[test]
fn a_manifest_alone_is_enough_to_be_a_node_project() {
    let fixture = Fixture::new("manifest-only");
    fixture.write("package.json", r#"{ "name": "tiny" }"#);

    let node = fixture.node();
    let package = node.manifest.package().expect("the manifest was read");
    assert_eq!(package.name.as_deref(), Some("tiny"));
    assert!(fixture.exists("package.json"), "the premise");
}

#[test]
fn a_lockfile_with_no_manifest_is_still_reported_as_a_node_project() {
    // The direction that must not be silent: SURE cannot read how this project
    // is built, and it should say that rather than say nothing.
    let fixture = Fixture::new("lockfile-only");
    fixture.write("yarn.lock", "# yarn lockfile v1\n");

    let found = fixture.discover();
    let report = found
        .report(Ecosystem::Node)
        .expect("yarn.lock is a Node file");
    assert_eq!(report.level, SupportLevel::InspectOnly);
    assert!(
        report.reason.contains("no package.json"),
        "the reason must say why SURE cannot go further: {}",
        report.reason
    );
    let node = fixture.node();
    assert!(node.manifest.is_absent());
    assert_eq!(node.managers.agreed(), Some(PackageManager::Yarn));
}

// ---------------------------------------------------------------------------
// Package managers
// ---------------------------------------------------------------------------

#[test]
fn the_package_manager_is_found_from_the_field_the_lockfile_and_the_engines_range() {
    let fixture = Fixture::new("managers");
    fixture.write(
        "package.json",
        r#"{
            "name": "managers",
            "packageManager": "pnpm@8.6.0",
            "engines": { "node": ">=20" }
        }"#,
    );
    fixture.write("pnpm-lock.yaml", "lockfileVersion: '6.0'\n");

    let node = fixture.node();
    let declared = node
        .managers
        .declared
        .as_ref()
        .expect("the field names one");
    assert_eq!(declared.manager, PackageManager::Pnpm);
    assert_eq!(
        declared.source.path,
        Path::new("package.json"),
        "a manager SURE names must come with the file that said so"
    );
    assert_eq!(node.managers.locked.len(), 1, "{:?}", node.managers.locked);
    assert_eq!(node.managers.agreed(), Some(PackageManager::Pnpm));
    assert!(node.managers.disagreement().is_none());
}

#[test]
fn a_project_that_names_one_manager_and_locks_another_is_reported_not_resolved() {
    // The whole reason there is no "pick the first lockfile" rule. A project
    // mid-migration from npm to pnpm has both, and an answer that picked one
    // would be confident and would be wrong half the time.
    let fixture = Fixture::new("disagreement");
    fixture.write(
        "package.json",
        r#"{ "name": "migrating", "packageManager": "pnpm@8.6.0" }"#,
    );
    fixture.write("package-lock.json", r#"{ "lockfileVersion": 3 }"#);

    let node = fixture.node();
    assert_eq!(
        node.managers.disagreement(),
        Some(Disagreement::DeclarationAndLockfile {
            declared: PackageManager::Pnpm,
            locked: PackageManager::Npm,
        })
    );
    assert_eq!(
        node.managers.agreed(),
        None,
        "SURE picked a winner where the project said two things"
    );
    assert!(
        !node
            .managers
            .disagreement()
            .unwrap()
            .plain_description()
            .is_empty(),
        "a disagreement a person cannot read is not a report"
    );
    // And both are still named, so a reader can see which two.
    assert_eq!(node.managers.locked.len(), 1);
    assert!(node.managers.declared.is_some());
}

#[test]
fn two_lockfiles_for_two_managers_are_a_disagreement() {
    let fixture = Fixture::new("two-lockfiles");
    fixture.write("package.json", r#"{ "name": "confused" }"#);
    fixture.write("yarn.lock", "# yarn lockfile v1\n");
    fixture.write("package-lock.json", r#"{ "lockfileVersion": 3 }"#);

    let node = fixture.node();
    match node.managers.disagreement() {
        Some(Disagreement::TwoLockfiles { first, second }) => {
            assert_ne!(
                first, second,
                "a disagreement between one manager and itself"
            );
        }
        other => panic!("expected TwoLockfiles, got {other:?}"),
    }
    assert_eq!(node.managers.agreed(), None);
}

#[test]
fn an_engines_range_alone_is_evidence_but_the_weakest_kind() {
    let fixture = Fixture::new("engines-only");
    fixture.write(
        "package.json",
        r#"{ "name": "hint", "engines": { "node": ">=20", "npm": ">=9" } }"#,
    );

    let node = fixture.node();
    assert!(node.managers.declared.is_none());
    assert!(node.managers.locked.is_empty());
    let mentioned: Vec<_> = node
        .managers
        .mentioned
        .iter()
        .map(|mention| (mention.manager, mention.range.as_str()))
        .collect();
    assert_eq!(mentioned, vec![(PackageManager::Npm, ">=9")]);
    assert_eq!(
        node.managers.agreed(),
        Some(PackageManager::Npm),
        "with nothing stronger, the hint is the answer"
    );
}

#[test]
fn a_manager_sure_does_not_recognise_is_not_reported_as_one_it_does() {
    let fixture = Fixture::new("unknown-manager");
    fixture.write(
        "package.json",
        r#"{ "name": "other", "packageManager": "corepack@0.20.0" }"#,
    );

    let node = fixture.node();
    assert!(
        node.managers.declared.is_none(),
        "SURE named a package manager it does not know"
    );
    assert_eq!(node.managers.agreed(), None);
}

// ---------------------------------------------------------------------------
// Declared scripts
// ---------------------------------------------------------------------------

#[test]
fn the_conventional_scripts_are_reported_including_the_ones_that_are_not_declared() {
    let fixture = Fixture::new("scripts");
    fixture.write(
        "package.json",
        r#"{
            "name": "scripts",
            "scripts": {
                "build": "tsc -p .",
                "test": "vitest run",
                "lint:fix": "eslint --fix ."
            }
        }"#,
    );

    let node = fixture.node();
    let rows = node
        .package()
        .expect("the manifest was read")
        .conventional_scripts();
    assert_eq!(rows.len(), ScriptRole::ALL.len());

    let build = script_named(&rows, ScriptRole::Build).expect("build is declared");
    assert_eq!(
        build.command, "tsc -p .",
        "the command must be carried verbatim, not summarised"
    );
    assert_eq!(
        script_named(&rows, ScriptRole::Test).unwrap().command,
        "vitest run"
    );

    // The rows that are absent are the point of the list.
    for role in [ScriptRole::Format, ScriptRole::Dev, ScriptRole::Clean] {
        assert!(
            script_named(&rows, role).is_none(),
            "{role:?} is not declared and must be reported as absent"
        );
    }
    // `lint:fix` is not `lint`. Mapping one onto the other would be SURE
    // inventing a convention the project did not follow.
    assert!(
        script_named(&rows, ScriptRole::Lint).is_none(),
        "an unconventional name was mapped onto a conventional role"
    );
    // And the script itself is still read, so nothing was lost by not mapping it.
    let package = node.package().unwrap();
    assert!(
        package
            .scripts
            .iter()
            .any(|script| script.name == "lint:fix"),
        "the declared script vanished from the manifest"
    );
}

#[test]
fn discovery_runs_none_of_the_scripts_it_reads() {
    // Reading a manifest is not executing a project. The command below would
    // leave `RAN.txt` beside the manifest if anything ran it through a shell,
    // and nothing here is allowed to.
    let fixture = Fixture::new("does-not-run");
    fixture.write(
        "package.json",
        r#"{
            "name": "runner",
            "scripts": {
                "build": "echo ran > RAN.txt",
                "test": "echo ran > RAN.txt",
                "prepare": "echo ran > RAN.txt"
            }
        }"#,
    );

    let node = fixture.node();
    let package = node.package().expect("the manifest was read");
    assert_eq!(
        package.scripts.len(),
        3,
        "all three scripts must be read for this test to mean anything"
    );
    assert!(
        !fixture.exists("RAN.txt"),
        "discovery executed a script it was only supposed to read"
    );
    // Belt and braces at the source level: this module tree must not be able to
    // start a process at all. **This catches the apparatus and not the
    // guarantee** — an implementation that shelled out through a helper in
    // another module would pass this and fail the check above.
    for file in ["discover/mod.rs", "discover/node.rs", "discover/read.rs"] {
        let source = std::fs::read_to_string(
            sure_testkit::repository_root()
                .join("crates/sure-core/src")
                .join(file),
        )
        .unwrap_or_else(|error| panic!("cannot read {file}: {error}"));
        for forbidden in ["process::Command", "std::process"] {
            assert!(
                !source.contains(forbidden),
                "{file} mentions {forbidden}, and discovery must not be able to \
                 start a process"
            );
        }
    }
}

#[test]
fn a_script_that_is_there_and_has_no_command_does_not_read_as_a_script_that_is_absent() {
    let fixture = Fixture::new("script-shape");
    fixture.write(
        "package.json",
        r#"{ "name": "odd", "scripts": { "test": ["jest"], "build": "tsc" } }"#,
    );

    let node = fixture.node();
    let package = node.package().expect("the manifest was read");
    assert!(
        script_named(&package.conventional_scripts(), ScriptRole::Test).is_none(),
        "a script whose value is not a command cannot be run"
    );
    assert_eq!(
        package.scripts_not_commands,
        vec!["test".to_owned()],
        "and the name must survive, or the report says the project declares no \
         test script when it declares an unusable one"
    );
}

#[test]
fn a_dependency_declared_with_no_range_does_not_read_as_a_dependency_that_is_absent() {
    // The dependency half of the rule the test above pins for scripts, and the
    // half that is easier to lose: a consumer looks a dependency up *by name*,
    // so a name that is dropped is indistinguishable from a name that was never
    // declared. `dependencies_not_ranges` is the only thing separating them.
    let fixture = Fixture::new("dependency-shape");
    fixture.write(
        "package.json",
        r#"{
            "name": "odd",
            "dependencies": { "left-pad": "^1.3.0" },
            "devDependencies": { "jest": ["29"], "typescript": "^5.0.0" },
            "peerDependencies": { "react": { "version": "18" } },
            "optionalDependencies": { "fsevents": 2 }
        }"#,
    );

    let node = fixture.node();
    let package = node.package().expect("the manifest was read");

    let read: Vec<(&str, &str)> = package
        .dependencies
        .iter()
        .map(|dependency| (dependency.name.as_str(), dependency.requirement.as_str()))
        .collect();
    assert_eq!(
        read,
        vec![("left-pad", "^1.3.0"), ("typescript", "^5.0.0")],
        "a dependency that does declare a range was dropped, or its range was \
         rewritten"
    );
    let left_pad = package
        .dependencies
        .iter()
        .find(|dependency| dependency.name == "left-pad")
        .expect("the runtime dependency was read");
    assert_eq!(
        left_pad.kind,
        DependencyKind::Runtime,
        "the section a dependency was declared in was lost"
    );

    assert_eq!(
        package.dependencies_not_ranges,
        vec!["fsevents".to_owned(), "jest".to_owned(), "react".to_owned()],
        "a dependency with no range must be named, or the report says the project \
         does not depend on it"
    );
}

#[test]
fn a_dependency_declared_with_no_range_in_two_sections_is_named_once() {
    // `scripts` is a single object with unique keys, so a script cannot repeat.
    // A dependency can: the same name is declared in several sections, and one
    // unusable name reachable from two of them is one thing wrong with the
    // project, not two.
    let fixture = Fixture::new("dependency-shape-twice");
    fixture.write(
        "package.json",
        r#"{
            "name": "odd",
            "dependencies": { "jest": ["29"] },
            "devDependencies": { "jest": ["29"] }
        }"#,
    );

    let node = fixture.node();
    let package = node.package().expect("the manifest was read");
    assert_eq!(
        package.dependencies_not_ranges,
        vec!["jest".to_owned()],
        "the name arrived once per section, and the report reads as two problems"
    );
}

// ---------------------------------------------------------------------------
// Workspace structure
// ---------------------------------------------------------------------------

#[test]
fn a_workspace_is_resolved_against_the_directories_that_are_really_there() {
    let fixture = Fixture::new("workspace");
    fixture.write(
        "package.json",
        r#"{
            "name": "root",
            "private": true,
            "workspaces": ["packages/*"],
            "devDependencies": { "turbo": "^1.10.0" }
        }"#,
    );
    fixture.write(
        "packages/api/package.json",
        r#"{ "name": "@root/api", "dependencies": { "express": "^4.18.0" } }"#,
    );
    fixture.write(
        "packages/web/package.json",
        r#"{
            "name": "@root/web",
            "dependencies": { "react": "^18.2.0", "next": "^14.0.0" },
            "devDependencies": { "vitest": "^1.0.0" },
            "scripts": { "dev": "next dev" }
        }"#,
    );
    // A directory the pattern does not match, to show the pattern is doing work.
    fixture.write("tools/package.json", r#"{ "name": "tools" }"#);

    // The premise, asked of the filesystem rather than of discovery.
    for member in ["packages/api", "packages/web", "tools"] {
        assert!(fixture.exists(member), "{member} is not there to be found");
    }

    let node = fixture.node();
    let workspaces = &node.workspaces;
    assert_eq!(workspaces.patterns, vec!["packages/*"]);
    assert!(workspaces.is_declared());
    assert!(!workspaces.truncated);

    let members: Vec<String> = workspaces
        .members
        .iter()
        .map(|member| member.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        members,
        vec!["packages/api".to_owned(), "packages/web".to_owned()],
        "the pattern names two directories and SURE listed neither or more"
    );
    for member in &workspaces.members {
        assert_eq!(member.manifest, MemberManifest::Present);
    }

    // Each member's own manifest was read, which is where a monorepo's real
    // dependencies live.
    let web = workspaces
        .members
        .iter()
        .find(|member| member.path.ends_with("web"))
        .expect("packages/web is a member");
    let web_package = web.package.as_ref().expect("its manifest was read");
    assert_eq!(web_package.name.as_deref(), Some("@root/web"));
    assert_eq!(
        web_package
            .command_for(PackageManager::Npm, ScriptRole::Dev)
            .as_deref(),
        Some("npm run dev")
    );

    // And the tooling from the members is in the project's list, which is the
    // point of reading them at all.
    let named: Vec<(&str, ToolRole, String)> = node
        .tooling
        .iter()
        .map(|tool| {
            (
                tool.package,
                tool.role,
                tool.from.to_string_lossy().replace('\\', "/"),
            )
        })
        .collect();
    assert!(
        named.contains(&("next", ToolRole::MetaFramework, "packages/web".to_owned())),
        "the root manifest declares none of the frameworks: {named:?}"
    );
    assert!(named.contains(&("react", ToolRole::WebFramework, "packages/web".to_owned())));
    assert!(named.contains(&(
        "express",
        ToolRole::ServerFramework,
        "packages/api".to_owned()
    )));
    assert!(named.contains(&("vitest", ToolRole::TestRunner, "packages/web".to_owned())));
    assert!(named.contains(&("turbo", ToolRole::MonorepoTool, String::new())));
    assert!(
        !named.iter().any(|(_, _, from)| from == "tools"),
        "a directory outside every pattern was read as a member"
    );
}

#[test]
fn the_root_is_never_a_member_of_its_own_workspace() {
    let fixture = Fixture::new("workspace-dot");
    fixture.write(
        "package.json",
        r#"{ "name": "root", "workspaces": [".", "packages/*"] }"#,
    );
    fixture.write("packages/app/package.json", r#"{ "name": "app" }"#);

    let node = fixture.node();
    let members: Vec<String> = node
        .workspaces
        .members
        .iter()
        .map(|member| member.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        members,
        vec!["packages/app".to_owned()],
        "the root's manifest is `NodeProject::manifest`, not a member of itself"
    );
}

#[test]
fn a_workspace_pattern_sure_does_not_expand_is_reported_rather_than_silently_dropped() {
    let fixture = Fixture::new("workspace-globstar");
    fixture.write(
        "package.json",
        r#"{ "name": "root", "workspaces": ["packages/**", "apps/*"] }"#,
    );
    fixture.write("packages/deep/inner/package.json", r#"{ "name": "inner" }"#);
    fixture.write("apps/web/package.json", r#"{ "name": "web" }"#);

    let node = fixture.node();
    assert!(
        node.workspaces
            .unresolved
            .iter()
            .any(|entry| entry.pattern == "packages/**"
                && entry.reason == UnresolvedReason::UnsupportedPattern),
        "a `**` pattern must be reported, because not expanding it quietly is how \
         a workspace comes to look smaller than it is: {:?}",
        node.workspaces.unresolved
    );
    // The pattern SURE does expand still works, so the refusal is about the one
    // form and not about workspaces.
    let members: Vec<String> = node
        .workspaces
        .members
        .iter()
        .map(|member| member.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(members, vec!["apps/web".to_owned()]);
}

#[test]
fn a_workspace_pattern_that_names_nothing_is_reported_as_naming_nothing() {
    let fixture = Fixture::new("workspace-nomatch");
    fixture.write(
        "package.json",
        r#"{ "name": "root", "workspaces": ["packages/*", "does-not-exist/*"] }"#,
    );
    fixture.mkdir("packages");

    let node = fixture.node();
    assert!(
        node.workspaces
            .unresolved
            .iter()
            .any(|entry| entry.pattern == "does-not-exist/*"
                && entry.reason == UnresolvedReason::NoMatch),
        "{:?}",
        node.workspaces.unresolved
    );
    assert!(
        node.workspaces
            .unresolved
            .iter()
            .any(|entry| entry.pattern == "packages/*"),
        "an empty `packages/` matches nothing, and that is a finding too: {:?}",
        node.workspaces.unresolved
    );
    assert!(node.workspaces.members.is_empty());
}

#[test]
fn a_workspace_pattern_cannot_reach_outside_the_project() {
    let fixture = Fixture::new("workspace-escape");
    fixture.write(
        "package.json",
        r#"{ "name": "root", "workspaces": ["../elsewhere/*", "packages/*"] }"#,
    );
    fixture.write("packages/app/package.json", r#"{ "name": "app" }"#);
    // A real sibling directory with a real manifest in it. A test whose
    // `../elsewhere` did not exist would pass whether or not the containment
    // rule worked.
    let sibling = fixture
        .path()
        .parent()
        .unwrap()
        .join("elsewhere")
        .join("evil");
    std::fs::create_dir_all(&sibling).expect("cannot create the sibling directory");
    std::fs::write(sibling.join("package.json"), r#"{ "name": "evil" }"#)
        .expect("cannot write the sibling manifest");
    assert!(sibling.join("package.json").exists(), "the premise");

    let node = fixture.node();
    assert!(
        node.workspaces
            .unresolved
            .iter()
            .any(|entry| entry.reason == UnresolvedReason::NotInsideProject),
        "{:?}",
        node.workspaces.unresolved
    );
    let members: Vec<String> = node
        .workspaces
        .members
        .iter()
        .map(|member| member.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        members,
        vec!["packages/app".to_owned()],
        "a pattern climbed out of the project and SURE followed it"
    );
}

#[test]
fn a_workspace_list_that_is_cut_short_says_so() {
    let fixture = Fixture::new("workspace-truncated");
    fixture.write(
        "package.json",
        r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
    );
    for name in ["a", "b", "c"] {
        fixture.write(
            &format!("packages/{name}/package.json"),
            &format!(r#"{{ "name": "{name}" }}"#),
        );
    }

    let node = fixture.node_with(DiscoverOptions::default().with_max_workspace_members(2));
    assert_eq!(node.workspaces.members.len(), 2);
    assert!(
        node.workspaces.truncated,
        "a member list that is a prefix of the real one must not read as the whole"
    );

    // And with room for all three, nothing is truncated — so the flag above is
    // about the limit and not about the flag being always on.
    let whole = fixture.node_with(DiscoverOptions::default().with_max_workspace_members(3));
    assert_eq!(whole.workspaces.members.len(), 3);
    assert!(!whole.workspaces.truncated);
}

#[test]
fn a_pnpm_workspace_file_is_read_when_the_manifest_declares_none() {
    let fixture = Fixture::new("pnpm-workspace");
    fixture.write("package.json", r#"{ "name": "root", "private": true }"#);
    fixture.write("pnpm-lock.yaml", "lockfileVersion: '6.0'\n");
    fixture.write("pnpm-workspace.yaml", "packages:\n  - 'libs/*'\n");
    fixture.write("libs/core/package.json", r#"{ "name": "@root/core" }"#);

    let node = fixture.node();
    assert_eq!(node.workspaces.patterns, vec!["libs/*"]);
    assert!(
        node.workspaces
            .declared_by
            .iter()
            .any(|source| source.path == Path::new("pnpm-workspace.yaml")),
        "the finding must name the file that declared the workspace: {:?}",
        node.workspaces.declared_by
    );
    let members: Vec<String> = node
        .workspaces
        .members
        .iter()
        .map(|member| member.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(members, vec!["libs/core".to_owned()]);
}

// ---------------------------------------------------------------------------
// Frameworks
// ---------------------------------------------------------------------------

#[test]
fn the_common_web_and_server_frameworks_are_recognised() {
    let fixture = Fixture::new("frameworks");
    fixture.write(
        "package.json",
        r#"{
            "name": "app",
            "dependencies": {
                "next": "^14.0.0",
                "react": "^18.2.0",
                "express": "^4.18.0",
                "some-unrecognised-package": "^1.0.0"
            },
            "devDependencies": {
                "@playwright/test": "^1.40.0",
                "@biomejs/biome": "^1.5.0",
                "typescript": "^5.3.0"
            }
        }"#,
    );

    let node = fixture.node();
    let by_role = |role: ToolRole| {
        node.tooling_of_role(role)
            .map(|tool| tool.package)
            .collect::<Vec<_>>()
    };
    assert_eq!(by_role(ToolRole::MetaFramework), vec!["next"]);
    assert_eq!(by_role(ToolRole::WebFramework), vec!["react"]);
    assert_eq!(by_role(ToolRole::ServerFramework), vec!["express"]);
    assert_eq!(by_role(ToolRole::EndToEndTest), vec!["@playwright/test"]);
    assert_eq!(by_role(ToolRole::Language), vec!["typescript"]);
    assert_eq!(
        by_role(ToolRole::Linter),
        vec!["@biomejs/biome"],
        "Biome lints and formats and must be reported as both"
    );
    assert_eq!(by_role(ToolRole::Formatter), vec!["@biomejs/biome"]);
    assert!(node.declares("express"));
    assert!(
        !node
            .tooling
            .iter()
            .any(|tool| tool.package.contains("unrecognised")),
        "an unrecognised package was guessed at rather than left silent"
    );
    // The section survives, so a reader can tell a runtime dependency from a
    // development one.
    let express = node
        .tooling
        .iter()
        .find(|tool| tool.package == "express")
        .expect("express was declared");
    assert_eq!(express.kind, DependencyKind::Runtime);
    assert!(express.package.is_ascii(), "a name from the table");
}

#[test]
fn a_dependency_declared_as_a_peer_is_reported_and_named_as_a_peer() {
    let fixture = Fixture::new("peer");
    fixture.write(
        "package.json",
        r#"{
            "name": "library",
            "peerDependencies": { "react": "^18.0.0" }
        }"#,
    );

    let node = fixture.node();
    let react = node
        .tooling
        .iter()
        .find(|tool| tool.package == "react")
        .expect("a peer dependency is still evidence about what this project is");
    assert_eq!(react.kind, DependencyKind::Peer);
}

// ---------------------------------------------------------------------------
// TypeScript
// ---------------------------------------------------------------------------

#[test]
fn typescript_is_detected_from_the_compiler_the_config_file_and_the_sources() {
    let fixture = Fixture::new("typescript");
    fixture.write(
        "package.json",
        r#"{ "name": "ts", "devDependencies": { "typescript": "^5.3.0" } }"#,
    );
    fixture.write(
        "tsconfig.json",
        r#"{ "compilerOptions": { "strict": true } }"#,
    );
    fixture.write("src/index.ts", "export const one: number = 1;\n");
    fixture.write("src/view.tsx", "export const View = () => null;\n");

    let node = fixture.node();
    assert!(node.typescript.is_present());
    let kinds: Vec<_> = node
        .typescript
        .evidence
        .iter()
        .map(|(kind, _)| *kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TypeScriptEvidence::Compiler,
            TypeScriptEvidence::Config,
            TypeScriptEvidence::SourceFiles,
        ],
        "every kind of evidence found must be reported, strongest first"
    );
    assert_eq!(node.typescript.source_files, 2);
    let (_, source) = &node.typescript.evidence[2];
    assert_eq!(
        source.path,
        Path::new("src/index.ts"),
        "the anchor must be the first source file in sorted order, so the finding \
         is a function of the project"
    );
}

#[test]
fn a_typescript_config_alone_says_this_is_a_project_sure_cannot_build() {
    let fixture = Fixture::new("tsconfig-only");
    fixture.write(
        "tsconfig.json",
        r#"{ "compilerOptions": { "strict": true } }"#,
    );
    fixture.write("main.ts", "export const one = 1;\n");

    let found = fixture.discover();
    let report = found
        .report(Ecosystem::Node)
        .expect("a tsconfig.json without a manifest still says TypeScript");
    assert_eq!(
        report.level,
        SupportLevel::InspectOnly,
        "there is no manifest, so there is nothing SURE can say about how to run it"
    );
    let node = fixture.node();
    assert!(node.manifest.is_absent());
    assert!(node.typescript.is_present());
    let kinds: Vec<_> = node
        .typescript
        .evidence
        .iter()
        .map(|(kind, _)| *kind)
        .collect();
    assert_eq!(
        kinds,
        vec![TypeScriptEvidence::Config, TypeScriptEvidence::SourceFiles]
    );
}

// ---------------------------------------------------------------------------
// A manifest that could not be read is not a manifest that is absent
// ---------------------------------------------------------------------------

#[test]
fn a_manifest_that_is_not_json_is_unread_and_never_absent() {
    let fixture = Fixture::new("bad-json");
    fixture.write("package.json", "{ this is not json");
    fixture.write("yarn.lock", "# yarn lockfile v1\n");

    // The premise, asserted with a tool that is not under test.
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("package.json")).unwrap(),
        "{ this is not json"
    );

    let found = fixture.discover();
    let report = found
        .report(Ecosystem::Node)
        .expect("there is a package.json here");
    assert_eq!(
        report.level,
        SupportLevel::InspectOnly,
        "SURE has not read the manifest, so it cannot say how the project is built"
    );
    assert!(
        report.reason.contains("could not read"),
        "the reason must be the unread one, not the absent one: {}",
        report.reason
    );

    let node = fixture.node();
    assert!(
        matches!(node.manifest, ManifestState::Unread(_)),
        "a package.json that failed to parse was reported as {:?}",
        node.manifest
    );
    // And it is in the result as a file SURE could not read, with a reason.
    let unread = found
        .unread
        .iter()
        .find(|entry| entry.path == Path::new("package.json"))
        .expect("the file SURE could not read must appear in the result");
    assert!(
        matches!(unread.reason, UnreadReason::NotParsed { .. }),
        "expected NotParsed, got {:?}",
        unread.reason
    );
    assert!(
        unread.reason.detail().is_some(),
        "the parser's words were dropped"
    );
    // The property, stated directly: this is not the absent answer.
    assert!(!node.manifest.is_absent());
    assert!(node.manifest.package().is_none());

    // A project with no manifest at all is a different finding and reads
    // differently, so the two cannot be confused by a reader either.
    let other = Fixture::new("no-manifest-at-all");
    other.write("yarn.lock", "# yarn lockfile v1\n");
    let other_node = other.node();
    assert!(other_node.manifest.is_absent());
    assert_ne!(
        report.reason,
        other
            .discover()
            .report(Ecosystem::Node)
            .expect("yarn.lock")
            .reason,
        "an unread manifest and an absent one must not read the same"
    );
}

#[test]
fn a_manifest_that_is_valid_json_and_not_a_manifest_is_unread_too() {
    // The second route to the same false green: `[1, 2, 3]` parses perfectly,
    // and read leniently it is a project that declares nothing.
    let fixture = Fixture::new("wrong-shape");
    fixture.write("package.json", "[1, 2, 3]");

    let found = fixture.discover();
    let node = fixture.node();
    match &node.manifest {
        ManifestState::Unread(reason) => assert!(
            matches!(reason, UnreadReason::WrongShape { found } if *found == "an array"),
            "expected WrongShape(an array), got {reason:?}"
        ),
        other => panic!("an array is not a package.json, got {other:?}"),
    }
    assert!(
        found
            .unread
            .iter()
            .any(|entry| entry.path == Path::new("package.json")),
        "the shape failure must be in the result"
    );
    assert_eq!(
        found.report(Ecosystem::Node).unwrap().level,
        SupportLevel::InspectOnly
    );
}

#[test]
fn a_manifest_sure_ran_out_of_budget_for_is_unread_and_never_absent() {
    // The third route: not a parse failure and not a shape failure, but a
    // project SURE declined to read. It answers the same question the other two
    // do, and it must not answer it differently.
    let fixture = Fixture::new("out-of-budget");
    fixture.write(
        "package.json",
        r#"{ "name": "unread", "scripts": { "build": "tsc" } }"#,
    );
    assert!(fixture.exists("package.json"), "the premise");

    let found = fixture.discover_with(DiscoverOptions::default().with_max_manifests(0));
    let node = match &found
        .report(Ecosystem::Node)
        .expect("the manifest is there")
        .findings
    {
        Findings::Node(node) => (**node).clone(),
    };
    match &node.manifest {
        ManifestState::Unread(UnreadReason::OutOfBudget { limit }) => assert_eq!(*limit, 0),
        other => panic!("expected OutOfBudget, got {other:?}"),
    }
    assert!(!node.manifest.is_absent());
    let unread = found
        .unread
        .iter()
        .find(|entry| entry.path == Path::new("package.json"))
        .expect("a file SURE did not read must be named");
    assert!(matches!(unread.reason, UnreadReason::OutOfBudget { .. }));
    // The lockfile summary is unaffected by the manifest budget, so a project
    // SURE could not read still gets its package manager where a lockfile says.
    assert_eq!(
        found.report(Ecosystem::Node).unwrap().level,
        SupportLevel::InspectOnly
    );
}

#[test]
fn a_file_too_large_to_read_is_unread_rather_than_half_read() {
    // Half a package.json is not a smaller package.json; it is a document that
    // fails to parse, and it would fail with a syntax error the project does not
    // have. So the limit refuses outright.
    let fixture = Fixture::new("too-large");
    let filler = "x".repeat(4_000);
    fixture.write(
        "package.json",
        &format!(r#"{{ "name": "big", "description": "{filler}" }}"#),
    );

    let found = fixture.discover_with(DiscoverOptions::default().with_max_manifest_bytes(1_000));
    let node = match &found
        .report(Ecosystem::Node)
        .expect("the file is there")
        .findings
    {
        Findings::Node(node) => (**node).clone(),
    };
    match &node.manifest {
        ManifestState::Unread(UnreadReason::TooLarge { limit }) => assert_eq!(*limit, 1_000),
        other => panic!("expected TooLarge, got {other:?}"),
    }
    let unread = found
        .unread
        .iter()
        .find(|entry| entry.path == Path::new("package.json"))
        .expect("the file SURE refused must be named");
    assert!(matches!(unread.reason, UnreadReason::TooLarge { .. }));
    // And a limit the file fits under reads it, so the refusal above is about
    // the limit rather than about large manifests being unreadable in general.
    let read = fixture.discover_with(DiscoverOptions::default().with_max_manifest_bytes(65_536));
    let node = match &read
        .report(Ecosystem::Node)
        .expect("the file is there")
        .findings
    {
        Findings::Node(node) => (**node).clone(),
    };
    assert_eq!(
        node.manifest
            .package()
            .and_then(|package| package.name.as_deref()),
        Some("big")
    );
}

#[test]
fn a_link_named_package_json_is_not_read_through() {
    // `read.rs`'s module comment says a link is never read through, and the
    // reason is containment: reading a link reads whatever the project points
    // at, which can be outside the project. Every other module that meets a link
    // refuses it — the walk does not follow one, the fingerprint records one by
    // its target — and a manifest read through one would be the single place in
    // the product that does not.
    //
    // The link here is a **directory** link, because a file symbolic link
    // cannot be created on this Windows host without Developer Mode or
    // administrator rights. What is therefore not covered is a link whose
    // target is a file; what is covered is that the name is answered from the
    // walk's own record of it rather than by opening it, which is one code path
    // for both — the walk's symlink arm runs before it looks at what is at the
    // other end.
    let fixture = Fixture::new("link-manifest");
    let outside = Fixture::new("link-manifest-target");
    outside.write(
        "package.json",
        r#"{ "name": "outside", "scripts": { "build": "OUTSIDE_MARKER" } }"#,
    );
    link_to_directory(outside.path(), &fixture.path().join("package.json"));

    // The premise, with the tools that are not under test: the manifest the link
    // points at is really there and really says what this test claims, and the
    // walk really recorded a link at the name rather than a file.
    let outside_manifest = std::fs::read_to_string(outside.path().join("package.json"))
        .expect("the manifest the link points at");
    assert!(
        outside_manifest.contains("OUTSIDE_MARKER"),
        "the fixture's own premise does not hold"
    );
    let walk = sure_core::scan::scan(fixture.path(), ScanOptions::default()).expect("a directory");
    assert!(
        walk.skipped().iter().any(|skipped| {
            skipped.path == Path::new("package.json") && skipped.reason == SkipReason::NotFollowed
        }),
        "the walk changed: it no longer records a link at this name"
    );

    let found = fixture.discover();
    let node = fixture.node();
    match &node.manifest {
        ManifestState::Unread(UnreadReason::NotReadableKind { kind }) => {
            assert!(kind.contains("link"), "the kind said {kind:?}");
        }
        other => panic!("expected the link to be unread, got {other:?}"),
    }
    // A link is a link, not an absence. The project must not be described as one
    // with no manifest, which is the same false statement as an unparsed file
    // arriving by a different route.
    assert!(!node.manifest.is_absent());
    let unread = found
        .unread
        .iter()
        .find(|entry| entry.path == Path::new("package.json"))
        .expect("the link SURE refused must be named");
    assert!(matches!(
        unread.reason,
        UnreadReason::NotReadableKind { .. }
    ));
    // And nothing the outside manifest said reached the result.
    assert!(
        node.package().is_none(),
        "a manifest behind a link was read: {:?}",
        node.package()
    );
    assert!(
        !format!("{node:?}").contains("OUTSIDE_MARKER"),
        "a value out of the linked manifest reached the findings"
    );
}

// ---------------------------------------------------------------------------
// What the walk decided, discovery must not quietly disagree with
// ---------------------------------------------------------------------------

#[test]
fn a_manifest_inside_an_ignored_directory_is_not_read() {
    // `node_modules` is left out of the walk, and discovery reads the walk. If
    // it reached into `node_modules` instead, an installed dependency's
    // `package.json` would be read as if the project had declared it — and a
    // dependency's own dependencies would become the project's.
    let fixture = Fixture::new("node-modules");
    fixture.write(
        "package.json",
        r#"{ "name": "root", "packageManager": "pnpm@8.6.0" }"#,
    );
    fixture.write(
        "node_modules/left-pad/package.json",
        r#"{ "name": "left-pad" }"#,
    );
    fixture.write("node_modules/left-pad/yarn.lock", "# yarn lockfile v1\n");
    fixture.write("node_modules/react/package.json", r#"{ "name": "react" }"#);

    // The premise: the file is really there, and the walk really leaves it out.
    assert!(fixture.exists("node_modules/react/package.json"));
    let walk = sure_core::scan::scan(fixture.path(), ScanOptions::default()).expect("a directory");
    assert!(
        walk.files()
            .all(|entry| !entry.path.starts_with("node_modules")),
        "the walk changed: it now lists node_modules"
    );

    let node = fixture.node();
    assert_eq!(
        node.managers.agreed(),
        Some(PackageManager::Pnpm),
        "a lockfile inside node_modules was read as the project's"
    );
    assert!(
        !node.declares("react"),
        "a dependency's own dependency was reported as the project's: {:?}",
        node.tooling
    );
    // The walk's decision is part of the result, so a reader can see it.
    assert!(
        node.tooling.is_empty(),
        "the project declares no dependencies of its own: {:?}",
        node.tooling
    );
}

#[test]
fn the_stacks_are_the_domains_own_vocabulary() {
    let fixture = Fixture::new("stacks");
    fixture.write("package.json", r#"{ "name": "app" }"#);

    let found = fixture.discover();
    let stacks = found.stacks();
    assert_eq!(stacks.len(), 1);
    assert_eq!(stacks[0].stack, "node");
    assert_eq!(stacks[0].level, SupportLevel::Generic);
    assert!(!stacks[0].reason.is_empty());
    // Derived from the reports rather than stored beside them, so the two cannot
    // disagree.
    assert_eq!(
        stacks[0].reason,
        found.report(Ecosystem::Node).unwrap().reason
    );
}

#[test]
fn the_conclusion_names_the_files_it_rests_on() {
    let fixture = Fixture::new("found-by");
    fixture.write("package.json", r#"{ "name": "app" }"#);
    fixture.write("package-lock.json", r#"{ "lockfileVersion": 3 }"#);
    fixture.write("tsconfig.json", "{}");

    let found = fixture.discover();
    let report = found.report(Ecosystem::Node).expect("a Node project");
    let named: Vec<String> = report
        .found_by
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        named,
        vec![
            "package.json".to_owned(),
            "package-lock.json".to_owned(),
            "tsconfig.json".to_owned(),
        ],
        "an anchorless conclusion is what the evidence model forbids"
    );
}

#[test]
fn discovery_is_a_function_of_the_project_and_not_of_the_run() {
    // Everything here is sorted or table-ordered, and the ways that goes wrong
    // are the ways a report becomes unstable. Two discoveries of one project
    // must agree, and a project that gained a file must be the only difference.
    let fixture = Fixture::new("deterministic");
    fixture.write(
        "package.json",
        r#"{
            "name": "app",
            "workspaces": ["packages/*"],
            "dependencies": { "react": "^18", "express": "^4" }
        }"#,
    );
    for name in ["a", "b", "c"] {
        fixture.write(
            &format!("packages/{name}/package.json"),
            &format!(r#"{{ "name": "{name}", "dependencies": {{ "vue": "^3" }} }}"#),
        );
    }

    let first = fixture.node();
    let second = fixture.node();
    assert_eq!(first, second, "two discoveries of one project disagreed");
    // An explicit assertion too, because `PartialEq` on a structure this large
    // would still pass if a field were dropped from it.
    let members: Vec<String> = first
        .workspaces
        .members
        .iter()
        .map(|member| member.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(members, vec!["packages/a", "packages/b", "packages/c"]);
}

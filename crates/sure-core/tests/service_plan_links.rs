//! `P18-T012`'s follow-up, checked from outside the crate: a declared service's
//! directory and entry are inside the project **by where they resolve to**, not
//! only by how they are spelled.
//!
//! # Why these two tests are in this directory rather than beside the module
//!
//! Everything else `service_plan.rs` decides is tested inside it, against real
//! directories, because what it decides is asked of the file system. These two
//! ask the same question and need one thing the others do not: **a directory
//! link**, which on Windows means starting `cmd` — `mklink /J` needs no
//! privilege where `std::os::windows::fs::symlink_dir` needs Developer Mode or
//! administrator rights, and `discover_node.rs` probed both on this host before
//! either fixture was written.
//!
//! `crates/sure-core/tests/spawn_sites.rs` counts every file under
//! `crates/*/src/` whose **code** builds a `std::process::Command`, and states
//! what the count means: *"Every one of them is a way for SURE to run
//! something."* A test fixture that makes a junction is not a way for SURE to
//! run something, and the census is narrowed to `src/` in its own words because
//! *"A test file is supposed to start processes."* So the helper that starts one
//! lives where the tests are, which is where the two junction fixtures that came
//! before it already were — and adding the file to that list instead would have
//! put a shipped-tree entry in it that ships no spawn at all.
//!
//! # What the two cases are
//!
//! `stays_inside` reads the text of a path. It is asked first, because a
//! declaration that spells `../outside` should be refused before anything is
//! asked of the disk, and it is right about everything it can see. It cannot see
//! a link: `escape` is one ordinary relative component, every textual test in the
//! module accepts it, and it is the **working directory** of the command SURE
//! would build — so the command would run a program outside the tree SURE was
//! handed, under a file the project chose the name of.
//!
//! The second case is the same link one component deeper: the directory is the
//! project root itself, the entry is a file that is really there, and the only
//! thing wrong with it is where it *is*. A check that only resolved the directory
//! would pass it.
//!
//! Both are checked against a link that leads **out** of the project, and the
//! fixture outside is a directory of its own rather than a path under the
//! project, because a link that led to somewhere inside would be a link the
//! planner is right to allow — a test built on one would pass with the
//! resolution check deleted.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use sure_core::config::{ChecksConfig, Launcher, ServiceDeclaration};
use sure_core::planned_work::CheckOperation;
use sure_core::schedule::{PlanBuilder, ScheduledCheck};
use sure_core::service_plan::{ServicePlan, ServiceRefusal};
use sure_domain::execution::{ExecutionMode, ExecutionPermissions};

fn scratch(name: &str) -> PathBuf {
    sure_testkit::scratch::directory("service plan links", name)
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
    }
    std::fs::write(path, text)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
}

/// A project with one `server.js` at the root and nothing else.
fn project(name: &str) -> PathBuf {
    let root = scratch(name);
    write(&root.join("server.js"), "// a service\n");
    root
}

/// A directory outside the project, for the link fixtures to lead to.
///
/// **A directory of its own and not a path under the project**, because the
/// fixture has to be one the planner should refuse to reach.
fn outside_project(name: &str) -> PathBuf {
    let outside = scratch(name);
    write(&outside.join("server.js"), "// somebody else's service\n");
    outside
}

/// A directory link at `link`, leading to `target`.
///
/// **A junction on Windows rather than a symbolic link**, because
/// `std::os::windows::fs::symlink_dir` needs Developer Mode or administrator
/// rights and `mklink /J` needs neither — `discover_node.rs` probed both on this
/// host before either fixture was written, and the answer there is the answer
/// here. That is not a weaker mechanism for the purpose: a junction is what a
/// link out of a working tree actually is on the platform SURE is developed on,
/// and it is created without any privilege, which is the whole reason the
/// planner resolves a path as well as reading it. The arguments are spelled with
/// backslashes because `mklink` reads `/` as the start of one of its own
/// switches.
#[cfg(windows)]
fn link_directory(target: &Path, link: &Path) {
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

/// A directory link at `link`, leading to `target`.
#[cfg(unix)]
fn link_directory(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap_or_else(|error| {
        panic!(
            "cannot link {} to {}: {error}",
            link.display(),
            target.display()
        )
    });
}

fn declaration(name: &str, port: u16) -> ServiceDeclaration {
    ServiceDeclaration {
        name: name.to_owned(),
        directory: None,
        launcher: Launcher::NodeEntry {
            entry: "server.js".to_owned(),
        },
        port,
        readiness: "/healthz".to_owned(),
        page: None,
    }
}

fn config(declarations: Vec<ServiceDeclaration>) -> ChecksConfig {
    ChecksConfig {
        services: declarations,
        ..ChecksConfig::default()
    }
}

/// The plan, as the schedule a builder made of it holds.
///
/// A builder for a run the mode and the user's permissions both allow SURE to
/// start the project's services in — so a check this plan holds would be built
/// and admitted rather than refused by the seam, and *nothing was planned* is
/// the plan's answer rather than the mode's.
fn planned(plan: &ServicePlan) -> Vec<ScheduledCheck> {
    let mut builder = PlanBuilder::new(
        ExecutionMode::HostConfirmed,
        ExecutionPermissions {
            run_project_code: true,
            ..ExecutionPermissions::inspect_only()
        },
    );
    plan.add_to(&mut builder);
    assert!(
        builder.refused().is_empty(),
        "every check this module builds must survive `PlanBuilder::propose`: {:?}",
        builder.refused()
    );
    let scheduled = builder.build().checks().to_vec();
    assert!(
        scheduled
            .iter()
            .all(|check| matches!(check.operation(), CheckOperation::Service(_))),
        "a declaration plans a service check or nothing"
    );
    scheduled
}

#[test]
fn a_declared_directory_that_is_a_link_out_of_the_project_is_refused() {
    // The case a textual check cannot see: `escape` is a plain relative path of
    // one ordinary component, so every textual test in the planner accepts it,
    // and it is the *working directory* of the command SURE would build — which
    // would be a program started outside the tree SURE was handed.
    let root = project("linked directory");
    link_directory(
        &outside_project("linked directory outside"),
        &root.join("escape"),
    );

    let mut declaration = declaration("api", 4310);
    declaration.directory = Some("escape".to_owned());
    let plan = ServicePlan::of(&root, &config(vec![declaration]));

    assert_eq!(
        plan.refusals().to_vec(),
        vec![ServiceRefusal::DirectoryOutsideTheProject {
            directory: "escape".to_owned(),
        }],
    );
    assert!(
        plan.services().is_empty() && planned(&plan).is_empty(),
        "a directory SURE will not run in is not a service SURE starts"
    );
}

#[test]
fn a_declared_entry_reached_through_a_link_is_refused() {
    // The same link, one component deeper: the directory is the project root
    // itself, the entry is a file that is there, and the only thing wrong with
    // it is where it *is*.
    let root = project("linked entry");
    link_directory(
        &outside_project("linked entry outside"),
        &root.join("escape"),
    );

    let mut declaration = declaration("api", 4310);
    declaration.launcher = Launcher::NodeEntry {
        entry: "escape/server.js".to_owned(),
    };
    let plan = ServicePlan::of(&root, &config(vec![declaration]));

    assert_eq!(
        plan.refusals().to_vec(),
        vec![ServiceRefusal::EntryIsOutsideTheProject {
            entry: "escape/server.js".to_owned(),
        }],
    );
    assert!(
        plan.services().is_empty() && planned(&plan).is_empty(),
        "a file SURE will not start is not a service SURE starts"
    );
}

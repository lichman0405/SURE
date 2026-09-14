//! The component graph end to end, from the outside.
//!
//! `crate::components`' own tests drive the graph through `scan` and `discover`
//! inside the crate. This file does the two things those cannot:
//!
//! 1. It uses the graph through the **public** API, so a shape that is only
//!    reachable inside the crate is not mistaken for a shape a caller can use.
//! 2. It asserts that `components.rs` **opens nothing**. That is the guarantee
//!    the module's own comment rests on — *"It is a view over a `Discovery` and
//!    opens no file, so it cannot disagree with the discovery it came from about
//!    what was in the project"* — and a guarantee stated in a comment and
//!    checked nowhere is exactly what
//!    `docs/architecture/EVIDENCE_MODEL.md` calls an unsupported claim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::components::{ComponentGraph, ManifestReading, Members, Stack};
use sure_core::discover::{DiscoverOptions, Ecosystem, discover};

/// A directory under the workspace's git-ignored `target/tmp`.
///
/// Unique per call and **never cleared**, which is the pattern this repository
/// settled on after a false report: clearing a fixed path with
/// `let _ = remove_dir_all(..)` and then treating it as fresh fails on Windows,
/// and the test then describes a directory that was never emptied. A path
/// nobody has used before needs no removal. Uniqueness comes from `create_dir`,
/// not from the name, so two processes given the same id cannot collide — a
/// directory that exists is skipped rather than adopted.
struct Fixture {
    project: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("components graph");
        std::fs::create_dir_all(&base)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
        for _ in 0..1_000 {
            let project = base.join(format!("{test}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
            match std::fs::create_dir(&project) {
                Ok(()) => return Self { project },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", project.display()),
            }
        }
        panic!("no free fixture name under {}", base.display());
    }

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

    fn graph(&self) -> ComponentGraph {
        let found = discover(&self.project, &DiscoverOptions::default()).expect("discover");
        ComponentGraph::of(&found)
    }
}

#[test]
fn a_monorepo_reads_as_several_components_from_the_public_api() {
    let fixture = Fixture::new("monorepo");
    fixture
        .write(
            "package.json",
            r#"{"name":"shop","workspaces":["apps/*","packages/*"]}"#,
        )
        .write("apps/web/package.json", r#"{"name":"web"}"#)
        .write("packages/ui/package.json", r#"{"name":"ui"}"#)
        .write("packages/db/package.json", r#"{"name":"db"}"#);

    let graph = fixture.graph();

    assert!(graph.is_multi_component());
    let members: Vec<&Path> = graph.members().map(|part| part.path.as_path()).collect();
    assert_eq!(
        members,
        vec![
            Path::new("apps/web"),
            Path::new("packages/db"),
            Path::new("packages/ui"),
        ]
    );
    // The root is a component too, and it is the one every member hangs from.
    assert_eq!(graph.root_component().stack(), Stack::Read);
    for member in graph.members() {
        assert_eq!(member.stack(), Stack::Read, "{}", member.path.display());
        assert_eq!(
            graph
                .contains
                .iter()
                .filter(|edge| edge.inner == member.path)
                .count(),
            1,
            "{} has exactly one container",
            member.path.display()
        );
    }
    assert_eq!(graph.not_read().count(), 0);
}

#[test]
fn a_member_sure_did_not_read_comes_back_as_an_inference_from_the_public_api() {
    let fixture = Fixture::new("unread-member");
    fixture
        .write(
            "package.json",
            r#"{"name":"shop","workspaces":["packages/*"]}"#,
        )
        .write("packages/ui/package.json", r#"{"name":"ui"}"#);

    // The task's second acceptance criterion, through the public API: a member
    // whose manifest was never read must not arrive as a stack. A budget of one
    // manifest is spent on the root, so the member's name is seen and not
    // opened.
    let found = discover(
        &fixture.project,
        &DiscoverOptions {
            max_manifests: 1,
            ..DiscoverOptions::default()
        },
    )
    .expect("discover");
    let graph = ComponentGraph::of(&found);

    let ui = graph
        .get(Path::new("packages/ui"))
        .expect("the member is still a component");
    assert_eq!(ui.stack(), Stack::Unknown);
    assert_eq!(
        ui.manifest_of(Ecosystem::Node)
            .expect("node named it")
            .reading,
        ManifestReading::NotOpened,
        "an unread manifest is not an absent one"
    );
    assert!(
        ui.plain_description().contains("inference"),
        "a component SURE did not read must be described as an inference: {}",
        ui.plain_description()
    );
    assert!(
        graph
            .not_read()
            .any(|part| part.path == Path::new("packages/ui")),
        "and it must be reachable as one of the things SURE could not tell you"
    );
}

#[test]
fn a_python_project_does_not_read_as_one_component_it_did_not_verify() {
    let fixture = Fixture::new("python-members");
    // A uv workspace: several packages, declared in a table SURE does not read.
    fixture
        .write(
            "pyproject.toml",
            "[project]\nname = \"shop\"\n\n[tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
        )
        .write("packages/ui/pyproject.toml", "[project]\nname = \"ui\"\n");

    let graph = fixture.graph();

    // The graph is one component wide, and that is exactly why the answer must
    // not be "this project has one component".
    assert!(!graph.is_multi_component());
    let python = graph
        .resolution_of(Ecosystem::Python)
        .expect("python has an entry");
    assert!(
        matches!(python.members, Members::NotRead { .. }),
        "{:?}",
        python.members
    );
    assert!(
        !python.members.is_known_single(),
        "SURE did not read the member list, so it cannot say this is one component"
    );
    assert!(
        graph.plain_description().contains("Python"),
        "the caveat must reach the graph's own sentence: {}",
        graph.plain_description()
    );
}

#[test]
fn the_component_graph_opens_no_file_and_starts_no_process() {
    // The guarantee this module is built on: it is a view over a `Discovery`.
    // If it opened a file it could disagree with the discovery it came from, and
    // a graph that disagreed with its own evidence would be the exact failure
    // this product exists to prevent.
    let source = std::fs::read_to_string(
        sure_testkit::repository_root().join("crates/sure-core/src/components.rs"),
    )
    .expect("read the module");

    // The tests inside the module write fixtures, so the module's own test half
    // is not part of the claim. Everything above the first `#[cfg(test)]` is the
    // code that ships. This is a cut at a known marker rather than a parse, and
    // it is stated as one: a second `#[cfg(test)]` further up would silently
    // widen what is searched, so the marker's absence is asserted below.
    let (production, tests) = source.split_once("#[cfg(test)]").unwrap_or_else(|| {
        panic!(
            "`components.rs` has no `#[cfg(test)]` marker, so this \
                                   check cannot tell the code that ships from the code \
                                   that writes fixtures, and it must not guess"
        )
    });
    assert!(
        !tests.contains("\n#[cfg(test)]"),
        "`components.rs` has a second `#[cfg(test)]`, and this check cuts at the \
         first — so part of the test half would have been searched as if it shipped"
    );

    // Comments are dropped, because **a mention is not an ability**: the module
    // comment explains that nothing here opens a file, and a check that read
    // that sentence as evidence of opening one would be red for the reason it
    // exists to be green.
    let code: String = production
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "File::open",
        "fs::read",
        "fs::write",
        "read_dir",
        "canonicalize",
        "metadata(",
        "process::Command",
        "std::process",
    ] {
        assert!(
            !code.contains(forbidden),
            "`components.rs` mentions {forbidden}, and the graph must be a view over \
             a discovery rather than something that reads the project again"
        );
    }
}

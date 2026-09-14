//! The production crate boundaries, asserted against the real manifests.
//!
//! P1-T001 acceptance: "Workspace compiles with acyclic dependencies."
//!
//! A check like this is only worth having if it can fail. Every assertion below
//! is paired with a case that feeds the checker a graph that *should* be
//! rejected, so a checker that always returned "no violations" would not pass.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sure_testkit::workspace::{self, DepKind, Edge};
use sure_testkit::{BoundaryPolicy, Crate, Workspace};

fn load() -> Workspace {
    Workspace::load(&sure_testkit::repository_root()).expect("workspace manifests must parse")
}

#[test]
fn the_declared_workspace_matches_the_boundary_policy() {
    let workspace = load();
    let policy = workspace::boundary_policy();
    let violations = workspace::violations(&workspace, &policy);
    assert!(
        violations.is_empty(),
        "crate boundaries were broken:\n{}",
        violations
            .iter()
            .map(|v| format!("  - {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_graph_reader_actually_sees_the_graph() {
    // Guards the check above from the way it would most plausibly become
    // useless: a parser that silently returns no edges reports every policy as
    // satisfied. Assert the reader found the members and edges that are known
    // to be in the manifests.
    let workspace = load();
    assert_eq!(
        workspace.member_names(),
        [
            "sure-cli",
            "sure-core",
            "sure-domain",
            "sure-protocol",
            "sure-testkit"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "member enumeration is wrong, so every policy result is untrustworthy"
    );

    let normal: Vec<String> = workspace.normal_edges().map(ToString::to_string).collect();
    for expected in [
        "sure-core -> sure-domain (dependencies)",
        "sure-core -> sure-protocol (dependencies)",
        "sure-cli -> sure-core (dependencies)",
        "sure-testkit -> sure-domain (dependencies)",
    ] {
        assert!(
            normal.contains(&expected.to_owned()),
            "expected edge '{expected}' was not read from the manifests; found {normal:?}"
        );
    }
}

#[test]
fn the_frozen_vocabulary_depends_on_nothing_inside_the_workspace() {
    // ADR 0010: the semantics crate must be changeable only on purpose. If it
    // could reach the engine or storage, a plumbing change could alter a stored
    // meaning without anyone deciding to.
    let workspace = load();
    let domain = workspace
        .crates
        .iter()
        .find(|c| c.name == "sure-domain")
        .expect("sure-domain is a workspace member");
    let internal: Vec<&Edge> = domain.edges.iter().collect();
    assert!(
        internal.is_empty(),
        "sure-domain gained internal dependencies: {internal:?}"
    );
}

#[test]
fn nothing_depends_on_the_testkit_in_production() {
    let workspace = load();
    let offenders: Vec<String> = workspace
        .normal_edges()
        .filter(|edge| edge.to == "sure-testkit")
        .map(ToString::to_string)
        .collect();
    assert!(
        offenders.is_empty(),
        "fixtures would ship inside the product: {offenders:?}"
    );
}

// --- the checker itself -------------------------------------------------

fn workspace_from(edges: &[(&str, &str)]) -> Workspace {
    let mut names: Vec<String> = Vec::new();
    for (from, to) in edges {
        for name in [*from, *to] {
            if !names.iter().any(|n| n == name) {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    Workspace {
        root: std::path::PathBuf::from("."),
        crates: names
            .into_iter()
            .map(|name| Crate {
                edges: edges
                    .iter()
                    .filter(|(from, _)| *from == name)
                    .map(|(from, to)| Edge {
                        from: (*from).to_owned(),
                        to: (*to).to_owned(),
                        kind: DepKind::Normal,
                    })
                    .collect(),
                dir: std::path::PathBuf::from(&name),
                name,
            })
            .collect(),
    }
}

#[test]
fn a_dependency_cycle_is_detected() {
    let cyc = workspace_from(&[("a", "b"), ("b", "c"), ("c", "a")]);
    assert_eq!(
        cyc.normal_cycle_crates(),
        ["a", "b", "c"].into_iter().map(str::to_owned).collect()
    );

    let acyclic = workspace_from(&[("a", "b"), ("b", "c")]);
    assert!(acyclic.normal_cycle_crates().is_empty());
}

#[test]
fn a_cycle_through_dev_dependencies_is_not_a_violation() {
    // Cargo permits this shape, and reporting it would train people to ignore
    // the check.
    let mut workspace = workspace_from(&[("a", "b")]);
    let a = workspace
        .crates
        .iter_mut()
        .find(|c| c.name == "a")
        .expect("a is present");
    a.edges.push(Edge {
        from: "a".to_owned(),
        to: "b".to_owned(),
        kind: DepKind::Dev,
    });
    let b = workspace
        .crates
        .iter_mut()
        .find(|c| c.name == "b")
        .expect("b is present");
    b.edges.push(Edge {
        from: "b".to_owned(),
        to: "a".to_owned(),
        kind: DepKind::Dev,
    });
    assert!(workspace.normal_cycle_crates().is_empty());
}

#[test]
fn an_edge_outside_the_policy_is_reported() {
    let workspace = workspace_from(&[("sure-domain", "sure-core")]);
    let policy = BoundaryPolicy {
        members: &["sure-domain", "sure-core"],
        allowed: &[("sure-domain", &[]), ("sure-core", &["sure-domain"])],
        no_internal_deps: &["sure-domain"],
        test_only: &[],
    };
    let violations = workspace::violations(&workspace, &policy);
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, workspace::Violation::NotStandalone { .. })),
        "a dependency out of the frozen vocabulary must be reported: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, workspace::Violation::DisallowedEdge { .. })),
        "the backward edge must also be reported as disallowed: {violations:?}"
    );
}

#[test]
fn a_member_with_no_policy_entry_is_reported() {
    // Otherwise a new crate could be added and silently exempted from every
    // rule, because an unlisted crate has no allowed-edges list to violate.
    let workspace = workspace_from(&[("newcomer", "sure-domain")]);
    let policy = workspace::boundary_policy();
    let violations = workspace::violations(&workspace, &policy);
    assert!(
        violations.iter().any(
            |v| matches!(v, workspace::Violation::UnlistedMember { name } if name == "newcomer")
        ),
        "an unlisted member must be reported: {violations:?}"
    );
}

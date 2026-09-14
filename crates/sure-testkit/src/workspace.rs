//! The repository's own crate dependency graph, parsed from the real manifests.
//!
//! P1-T001 fixes the production crate boundaries. A boundary that exists only
//! in a document erodes the first time a dependency is added for convenience,
//! so the boundary is also a test: this module parses every workspace manifest,
//! builds the graph that Cargo will actually build, and reports departures from
//! [`BoundaryPolicy`].
//!
//! The graph is read with a real TOML parser rather than a hand-rolled one. A
//! parser that silently mis-reads a manifest would report "no violations",
//! which is a false green inside the check that exists to prevent false greens.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

/// Which dependency table an edge came from.
///
/// The distinction is load-bearing, not cosmetic: Cargo permits a cycle through
/// `dev-dependencies` (a crate may dev-depend on something that depends on it),
/// so acyclicity is a property of normal edges only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepKind {
    /// `[dependencies]`. Compiled into the crate and into everything using it.
    Normal,
    /// `[dev-dependencies]`. Test-only; may close a cycle with a normal edge.
    Dev,
    /// `[build-dependencies]`. Runs at build time.
    Build,
}

impl DepKind {
    /// The manifest table this kind is read from.
    #[must_use]
    pub const fn table(self) -> &'static str {
        match self {
            Self::Normal => "dependencies",
            Self::Dev => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }

    /// Whether an edge of this kind is part of the shipped dependency graph.
    #[must_use]
    pub const fn is_compiled_into_dependents(self) -> bool {
        matches!(self, Self::Normal)
    }
}

/// One internal dependency: `from` depends on `to`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    /// The depending crate.
    pub from: String,
    /// The depended-on crate.
    pub to: String,
    /// Which dependency table the edge was read from.
    pub kind: DepKind,
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {} ({})", self.from, self.to, self.kind.table())
    }
}

/// One workspace member and its internal edges.
#[derive(Debug, Clone)]
pub struct Crate {
    /// The `[package] name`.
    pub name: String,
    /// The directory holding the manifest.
    pub dir: PathBuf,
    /// Internal edges declared by this crate.
    pub edges: Vec<Edge>,
}

impl Crate {
    /// Internal edges of one kind.
    pub fn edges_of_kind(&self, kind: DepKind) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.kind == kind)
    }
}

/// The parsed workspace.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Repository root, resolved.
    pub root: PathBuf,
    /// Every member, sorted by name.
    pub crates: Vec<Crate>,
}

/// Failures that stop the graph from being read at all.
///
/// These are distinct from policy [`Violation`]s: an unreadable manifest is a
/// broken check, not a clean result.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A manifest was not valid TOML.
    Toml {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        source: toml::de::Error,
    },
    /// The root manifest has no `[workspace] members` array.
    NoWorkspace {
        /// The root manifest.
        path: PathBuf,
    },
    /// A member entry is a glob, which this reader does not expand.
    ///
    /// Reported rather than skipped: treating an unexpanded glob as "no
    /// members" would silently shrink the graph and hide violations.
    UnsupportedMemberGlob {
        /// The member entry as written.
        entry: String,
    },
    /// A `path` dependency did not resolve to a workspace member.
    UnknownPathDependency {
        /// The depending crate.
        from: String,
        /// The path as resolved.
        path: PathBuf,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Toml { path, source } => write!(f, "cannot parse {}: {source}", path.display()),
            Self::NoWorkspace { path } => {
                write!(f, "{} has no [workspace] members array", path.display())
            }
            Self::UnsupportedMemberGlob { entry } => write!(
                f,
                "workspace member entry '{entry}' is a glob; expand it or list members explicitly"
            ),
            Self::UnknownPathDependency { from, path } => write!(
                f,
                "{from} depends on a path that is not a workspace member: {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Toml { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Parse a manifest as a TOML document.
///
/// A document is a `Table`, not a `Value`: TOML 1.x reads a `Value` as a single
/// value, so parsing a manifest into one fails on the second key rather than on
/// anything meaningful.
fn read_manifest(path: &Path) -> Result<toml::Table, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str::<toml::Table>(&text).map_err(|source| Error::Toml {
        path: path.to_path_buf(),
        source,
    })
}

/// Resolve a directory for comparison, tolerating a missing path.
///
/// `canonicalize` fails on a path that does not exist; falling back to the
/// literal join keeps the error about the missing member rather than about the
/// canonicalisation.
fn resolve(dir: &Path) -> PathBuf {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

/// Everything needed to turn a `path` dependency into an internal edge.
struct Resolver<'a> {
    base_dir: PathBuf,
    by_dir: &'a BTreeMap<PathBuf, String>,
    workspace_deps: Option<&'a toml::Value>,
}

impl Resolver<'_> {
    /// Append the internal edges declared in one dependency table.
    fn edges_from_table(
        &self,
        table: &toml::Value,
        from: &str,
        kind: DepKind,
        member_dir: &Path,
        out: &mut Vec<Edge>,
    ) -> Result<(), Error> {
        let Some(entries) = table.as_table() else {
            return Ok(());
        };
        for (name, spec) in entries {
            let direct = spec.get("path").and_then(toml::Value::as_str);
            let via_workspace = || {
                // `crate = { workspace = true }` carries no path here; the path
                // lives in the root `[workspace.dependencies]`. Missing this
                // would hide an edge, so it is resolved explicitly.
                let uses_workspace = spec
                    .get("workspace")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false);
                if !uses_workspace {
                    return None;
                }
                self.workspace_deps?.get(name)?.get("path")?.as_str()
            };
            let Some(path) = direct.or_else(via_workspace) else {
                continue;
            };
            let target = resolve(&member_dir.join(path));
            match self.by_dir.get(&target) {
                Some(to) => out.push(Edge {
                    from: from.to_owned(),
                    to: to.clone(),
                    kind,
                }),
                None => {
                    // A path dependency that stays inside the repository but
                    // names no member is a broken manifest. Reported rather
                    // than dropped: silently ignoring it would shrink the graph
                    // and hide whatever else is wrong with it.
                    if target.starts_with(&self.base_dir) {
                        return Err(Error::UnknownPathDependency {
                            from: from.to_owned(),
                            path: target,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

impl Workspace {
    /// Parse the workspace rooted at `root` (the directory holding `Cargo.toml`).
    pub fn load(root: &Path) -> Result<Self, Error> {
        let root_manifest = root.join("Cargo.toml");
        let root_value = read_manifest(&root_manifest)?;
        let workspace = root_value
            .get("workspace")
            .ok_or_else(|| Error::NoWorkspace {
                path: root_manifest.clone(),
            })?;
        let member_entries = workspace
            .get("members")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| Error::NoWorkspace {
                path: root_manifest.clone(),
            })?;

        // Pass 1: directory -> crate name, so path dependencies can be resolved
        // regardless of declaration order.
        let mut dirs: Vec<PathBuf> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        for entry in member_entries {
            let Some(entry) = entry.as_str() else {
                continue;
            };
            if entry.contains('*') {
                return Err(Error::UnsupportedMemberGlob {
                    entry: entry.to_owned(),
                });
            }
            let dir = root.join(entry);
            let manifest = read_manifest(&dir.join("Cargo.toml"))?;
            let name = manifest
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(toml::Value::as_str)
                .unwrap_or(entry)
                .to_owned();
            names.push(name);
            dirs.push(resolve(&dir));
        }
        let by_dir: BTreeMap<PathBuf, String> =
            dirs.iter().cloned().zip(names.iter().cloned()).collect();

        let workspace_deps = workspace.get("dependencies");
        let root_resolved = resolve(root);
        let resolver = Resolver {
            base_dir: root_resolved.clone(),
            by_dir: &by_dir,
            workspace_deps,
        };
        let tables: [(&str, DepKind); 3] = [
            ("dependencies", DepKind::Normal),
            ("dev-dependencies", DepKind::Dev),
            ("build-dependencies", DepKind::Build),
        ];

        // Pass 2: edges.
        let mut crates = Vec::with_capacity(names.len());
        for (dir, name) in dirs.iter().zip(names.iter()) {
            let manifest = read_manifest(&dir.join("Cargo.toml"))?;
            let mut edges = Vec::new();
            for (table_name, kind) in tables {
                if let Some(table) = manifest.get(table_name) {
                    resolver.edges_from_table(table, name, kind, dir, &mut edges)?;
                }
            }
            // `[target.'cfg(...)'.dependencies]` is a real dependency edge for
            // the targets it applies to, so it belongs in the graph.
            if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
                for target in targets.values() {
                    for (table_name, kind) in tables {
                        if let Some(table) = target.get(table_name) {
                            resolver.edges_from_table(table, name, kind, dir, &mut edges)?;
                        }
                    }
                }
            }
            edges.sort();
            crates.push(Crate {
                name: name.clone(),
                dir: dir.clone(),
                edges,
            });
        }
        crates.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self {
            root: root_resolved,
            crates,
        })
    }

    /// Member names, sorted.
    #[must_use]
    pub fn member_names(&self) -> BTreeSet<String> {
        self.crates.iter().map(|c| c.name.clone()).collect()
    }

    /// Every internal edge of every kind.
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.crates.iter().flat_map(|c| c.edges.iter())
    }

    /// Normal edges only — the shipped dependency graph.
    pub fn normal_edges(&self) -> impl Iterator<Item = &Edge> {
        self.crates
            .iter()
            .flat_map(|c| c.edges_of_kind(DepKind::Normal))
    }

    /// Crates participating in a cycle of normal edges.
    ///
    /// Returns the crates left over after repeatedly removing everything with
    /// no remaining incoming edge; those are exactly the crates on or
    /// downstream of a cycle.
    #[must_use]
    pub fn normal_cycle_crates(&self) -> BTreeSet<String> {
        let mut incoming: BTreeMap<&str, usize> = self
            .crates
            .iter()
            .map(|c| (c.name.as_str(), 0usize))
            .collect();
        let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in self.normal_edges() {
            if let Some(count) = incoming.get_mut(edge.to.as_str()) {
                *count += 1;
            }
            outgoing
                .entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }
        let mut ready: Vec<&str> = incoming
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(name, _)| *name)
            .collect();
        let mut settled: BTreeSet<&str> = BTreeSet::new();
        while let Some(name) = ready.pop() {
            if !settled.insert(name) {
                continue;
            }
            for next in outgoing.get(name).into_iter().flatten() {
                if let Some(count) = incoming.get_mut(next) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push(next);
                    }
                }
            }
        }
        incoming
            .into_keys()
            .filter(|name| !settled.contains(name))
            .map(str::to_owned)
            .collect()
    }
}

/// The crate boundaries P1-T001 fixes.
///
/// `allowed` is an upper bound, not a required set: a crate may depend on fewer
/// crates than it is allowed to. A crate that is absent from `allowed` may not
/// depend on any other member.
///
/// `sure-testkit` is the one deliberate exception to layering: it is test
/// support, so it may read the frozen vocabulary and the protocol, but nothing
/// may depend on it in production, or fixtures would ship inside the product.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryPolicy {
    /// Crates that must exist in the workspace.
    pub members: &'static [&'static str],
    /// Maximum allowed normal-dependency edges, per crate.
    pub allowed: &'static [(&'static str, &'static [&'static str])],
    /// Crates that must have no internal normal dependency at all.
    pub no_internal_deps: &'static [&'static str],
    /// Crates that must never appear as another crate's normal dependency.
    pub test_only: &'static [&'static str],
}

/// The policy as it stands for v0.1.
#[must_use]
pub const fn boundary_policy() -> BoundaryPolicy {
    BoundaryPolicy {
        members: &[
            "sure-domain",
            "sure-protocol",
            "sure-core",
            "sure-testkit",
            "sure-cli",
        ],
        allowed: &[
            // The frozen vocabulary. Depends on nothing inside the workspace,
            // so a plumbing change cannot alter a stored meaning (ADR 0010).
            ("sure-domain", &[]),
            ("sure-protocol", &["sure-domain"]),
            ("sure-core", &["sure-domain", "sure-protocol"]),
            ("sure-testkit", &["sure-domain", "sure-protocol"]),
            ("sure-cli", &["sure-core", "sure-protocol", "sure-domain"]),
        ],
        no_internal_deps: &["sure-domain"],
        test_only: &["sure-testkit"],
    }
}

/// A departure from the declared boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A crate the policy names is not a workspace member.
    MissingMember {
        /// The crate name.
        name: String,
    },
    /// A workspace member the policy does not list.
    UnlistedMember {
        /// The crate name.
        name: String,
    },
    /// A crate depends on another crate the policy does not allow.
    DisallowedEdge {
        /// The offending edge.
        edge: Edge,
    },
    /// A crate that must stand alone has an internal normal dependency.
    NotStandalone {
        /// The offending edge.
        edge: Edge,
    },
    /// Something depends on a test-only crate in production.
    TestOnlyInProduction {
        /// The offending edge.
        edge: Edge,
    },
    /// Normal dependencies form a cycle, which will not compile.
    NormalCycle {
        /// The crates involved.
        crates: Vec<String>,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMember { name } => {
                write!(f, "policy names '{name}', which is not a workspace member")
            }
            Self::UnlistedMember { name } => write!(
                f,
                "'{name}' is a workspace member with no entry in BoundaryPolicy::allowed"
            ),
            Self::DisallowedEdge { edge } => write!(
                f,
                "{} is not an allowed production dependency; update BoundaryPolicy and the ADR \
                 together if the boundary is genuinely changing",
                edge
            ),
            Self::NotStandalone { edge } => write!(
                f,
                "{} breaks the guarantee that the frozen vocabulary stands alone",
                edge
            ),
            Self::TestOnlyInProduction { edge } => write!(
                f,
                "{}: test support must not ship inside the product; use a dev-dependency",
                edge
            ),
            Self::NormalCycle { crates } => write!(
                f,
                "normal dependencies form a cycle among: {}",
                crates.join(", ")
            ),
        }
    }
}

/// Check a parsed workspace against a policy.
///
/// Returns every violation rather than the first, so one run reports the whole
/// state of the boundary.
#[must_use]
pub fn violations(workspace: &Workspace, policy: &BoundaryPolicy) -> Vec<Violation> {
    let mut found = Vec::new();
    let members = workspace.member_names();

    for name in policy.members {
        if !members.contains(*name) {
            found.push(Violation::MissingMember {
                name: (*name).to_owned(),
            });
        }
    }
    for member in &members {
        if !policy
            .allowed
            .iter()
            .any(|(name, _)| *name == member.as_str())
        {
            found.push(Violation::UnlistedMember {
                name: member.clone(),
            });
        }
    }

    for edge in workspace.normal_edges() {
        if policy.no_internal_deps.contains(&edge.from.as_str()) {
            found.push(Violation::NotStandalone { edge: edge.clone() });
        }
        if policy.test_only.contains(&edge.to.as_str()) {
            found.push(Violation::TestOnlyInProduction { edge: edge.clone() });
        }
        let permitted = policy
            .allowed
            .iter()
            .find(|(name, _)| *name == edge.from)
            .map(|(_, allowed)| *allowed)
            .unwrap_or(&[]);
        if !permitted.contains(&edge.to.as_str()) {
            found.push(Violation::DisallowedEdge { edge: edge.clone() });
        }
    }

    let cycle = workspace.normal_cycle_crates();
    if !cycle.is_empty() {
        found.push(Violation::NormalCycle {
            crates: cycle.into_iter().collect(),
        });
    }

    found
}

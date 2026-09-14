//! What a JavaScript or TypeScript project declares about itself.
//!
//! See [`super`] for the rule this is all arranged around: **a manifest states a
//! request, not a fact.** Nothing here runs `npm`, resolves a version, or opens
//! `node_modules`.
//!
//! # The files it reads, and why those
//!
//! | File | What it answers |
//! |---|---|
//! | `package.json` | the whole of what the project declares |
//! | `package-lock.json`, `npm-shrinkwrap.json`, `yarn.lock`, `pnpm-lock.yaml`, `bun.lockb`, `bun.lock` | which package manager was used |
//! | `pnpm-workspace.yaml` | workspace members, for the one manager that does not put them in `package.json` |
//! | `tsconfig.json`, `jsconfig.json` | that this is a TypeScript project, when the manifest does not say |
//!
//! A lockfile is **not parsed**. Its contents are a resolved dependency graph
//! SURE has no use for and that would cost megabytes to read; what is wanted
//! from it is one bit — *this project has a `pnpm-lock.yaml`* — and that bit is
//! the file's existence. Which is also why the byte budget is never spent on one.
//!
//! # Why the package manager is not simply "the one with a lockfile"
//!
//! Because projects get this wrong, and SURE reporting a confident answer where
//! the project contradicts itself is worse than SURE reporting the
//! contradiction. Three kinds of evidence are collected and kept apart:
//!
//! - **declared** — the `packageManager` field. Corepack reads it, so it is the
//!   project's own statement about itself and the strongest thing available.
//! - **locked** — a lockfile exists for that manager.
//! - **mentioned** — an `engines` range names it, which is a hint about what a
//!   contributor should install and nothing more.
//!
//! [`Managers::disagreement`] reports the two cases where the evidence points two
//! ways, and [`Managers::agreed`] answers `None` for both rather than picking a
//! winner. There is no code path here that silently prefers one lockfile to
//! another.
//!
//! # Why the tool table is fixed
//!
//! Package names live in the project's manifest, so a finding built from one
//! would let a project write text into a sentence a person reads. Every tool
//! this module reports comes from [`TOOLS`], and the value stored is the table's
//! own `&'static str` — the project's spelling is never what is reported. A
//! dependency that is not in the table is not reported at all, which is a
//! deliberate silence: "SURE did not recognise this package" is not a finding,
//! and guessing from a name would be.

use std::path::{Path, PathBuf};

use sure_domain::vocabulary::SupportLevel;

use super::pattern;
use super::read::{self, Budget, Probe, ReadFile, UnreadReason};
use super::{DiscoverOptions, Ecosystem, EcosystemReport, Findings, Source, Unread};
use crate::scan::Scan;

/// Re-exported so that `node::MemberManifest` keeps naming the type it always
/// named. It moved to [`super`] when Cargo workspaces became the second
/// ecosystem to describe a member's manifest by probing for it, and one member
/// directory described as `Present` by one module and `NotReadable` by another
/// would be the same directory given two answers.
pub use super::MemberManifest;

/// Re-exported so that `node::UnresolvedReason` keeps naming the type it always
/// named. The resolution itself moved to [`super::pattern`] when Cargo
/// workspaces became the second ecosystem to declare members with globs, and two
/// ecosystems expanding `*` differently would report two different workspaces for
/// one directory tree.
pub use super::pattern::{Unresolved, UnresolvedReason};

/// The manifest every Node project has, relative to the project root.
const MANIFEST: &str = "package.json";

/// The lockfiles recognised, each naming the manager that writes it.
///
/// A table rather than a chain of `if`s so that "every lockfile SURE knows" is a
/// list a test can hold against [`PackageManager::ALL`]: a manager that could
/// never be detected from a lockfile is worth failing a test over.
const LOCKFILES: &[(&str, PackageManager)] = &[
    ("package-lock.json", PackageManager::Npm),
    ("npm-shrinkwrap.json", PackageManager::Npm),
    ("yarn.lock", PackageManager::Yarn),
    ("pnpm-lock.yaml", PackageManager::Pnpm),
    // Bun renamed its lockfile in 1.2. Both are in use and a project carries at
    // most one, so listing both is not a conflict.
    ("bun.lockb", PackageManager::Bun),
    ("bun.lock", PackageManager::Bun),
];

/// The file pnpm keeps its workspace list in.
const PNPM_WORKSPACE: &str = "pnpm-workspace.yaml";

/// The files that say "this is a TypeScript project" without the manifest
/// saying so.
const TSCONFIGS: &[&str] = &["tsconfig.json", "jsconfig.json"];

/// The extensions counted as TypeScript source, without their dots.
const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts"];

/// A package manager SURE can recognise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageManager {
    /// `npm`.
    Npm,
    /// `yarn`.
    Yarn,
    /// `pnpm`.
    Pnpm,
    /// `bun`.
    Bun,
}

impl PackageManager {
    /// Every manager SURE recognises, in a fixed order.
    pub const ALL: &'static [Self] = &[Self::Npm, Self::Yarn, Self::Pnpm, Self::Bun];

    /// The stable name, and the command a person would type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
            Self::Bun => "bun",
        }
    }

    /// The manager a name refers to, if SURE recognises it.
    ///
    /// The lookup direction that matters: a name arrives as data from a
    /// manifest, and this is the only way it becomes a [`PackageManager`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|known| known.as_str() == name)
    }
}

/// How SURE came to name a package manager.
///
/// Ordered strongest first, which is the order [`Managers::agreed`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ManagerEvidence {
    /// The `packageManager` field named it.
    Declared,
    /// A lockfile for it is present.
    Lockfile,
    /// An `engines` range named it.
    EnginesRange,
}

impl ManagerEvidence {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Lockfile => "lockfile",
            Self::EnginesRange => "engines_range",
        }
    }

    /// The sentence a person reads.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Declared => "the project names it in the packageManager field",
            Self::Lockfile => "this project has a lockfile for it",
            Self::EnginesRange => {
                "the project's engines field asks for a version of it, which is a hint \
                 rather than a decision"
            }
        }
    }
}

/// One package manager, and why SURE named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerFinding {
    /// Which manager.
    pub manager: PackageManager,
    /// How SURE knows.
    pub evidence: ManagerEvidence,
    /// The file it came from.
    pub source: Source,
}

/// Two pieces of evidence naming two different managers.
///
/// Reported rather than resolved. The project is the authority on what it uses,
/// and where the project says two things SURE's answer is that it says two
/// things — not a coin toss that will be wrong half the time and confident
/// every time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disagreement {
    /// Two lockfiles, for two managers.
    TwoLockfiles {
        /// The first, by the order the lockfiles were met.
        first: PackageManager,
        /// The second.
        second: PackageManager,
    },
    /// The `packageManager` field names one manager and a lockfile names
    /// another.
    DeclarationAndLockfile {
        /// What the project says it uses.
        declared: PackageManager,
        /// What a lockfile says was used.
        locked: PackageManager,
    },
}

impl Disagreement {
    /// The sentence a person reads.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::TwoLockfiles { .. } => {
                "This project has lockfiles for two different package managers, so SURE \
                 cannot tell which one it uses."
            }
            Self::DeclarationAndLockfile { .. } => {
                "This project names one package manager and has a lockfile from another, \
                 so SURE cannot tell which one it uses."
            }
        }
    }

    /// One of the two managers involved.
    #[must_use]
    pub const fn one_of(self) -> PackageManager {
        match self {
            Self::TwoLockfiles { first, .. } => first,
            Self::DeclarationAndLockfile { declared, .. } => declared,
        }
    }

    /// The other one.
    #[must_use]
    pub const fn other_of(self) -> PackageManager {
        match self {
            Self::TwoLockfiles { second, .. } => second,
            Self::DeclarationAndLockfile { locked, .. } => locked,
        }
    }
}

/// A package manager named by an `engines` range.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManagerMention {
    /// Which manager the field named.
    pub manager: PackageManager,
    /// The range, **verbatim**. SURE does not interpret it.
    pub range: String,
}

/// Every manager SURE found evidence for, kept by how strong the evidence is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Managers {
    /// The manager the `packageManager` field names, if it names one.
    pub declared: Option<ManagerFinding>,
    /// Managers a lockfile names. More than one is a disagreement.
    pub locked: Vec<ManagerFinding>,
    /// Managers an `engines` range names.
    pub mentioned: Vec<ManagerMention>,
}

impl Managers {
    /// Where the evidence contradicts itself, if it does.
    ///
    /// The declaration is checked first: a project that names pnpm and carries a
    /// `package-lock.json` is mid-migration, and that is a more useful thing to
    /// say than "there are two lockfiles" even when both are true.
    #[must_use]
    pub fn disagreement(&self) -> Option<Disagreement> {
        let declared = self.declared.as_ref().map(|found| found.manager);
        let mut locked = self.locked.iter().map(|found| found.manager);
        let first = locked.next();

        match (declared, first) {
            (Some(declared), Some(locked)) if declared != locked => {
                Some(Disagreement::DeclarationAndLockfile { declared, locked })
            }
            (_, Some(first)) => locked
                .find(|&found| found != first)
                .map(|second| Disagreement::TwoLockfiles { first, second }),
            // One lockfile that matches the declaration, or no declaration at
            // all, or nothing locked: every one of these is evidence pointing
            // one way, and the two `None`s in it mean different things that
            // `agreed` keeps apart by asking this method first.
            (Some(_), None) | (None, None) => None,
        }
    }

    /// The one manager SURE would use, or `None` when it cannot pick one.
    ///
    /// `None` for two different reasons, and they are not the same finding: no
    /// evidence at all, and evidence pointing two ways. A caller that renders
    /// the two the same way is throwing away [`Self::disagreement`], which is
    /// the part a person needs.
    #[must_use]
    pub fn agreed(&self) -> Option<PackageManager> {
        if self.disagreement().is_some() {
            return None;
        }
        self.declared
            .as_ref()
            .map(|found| found.manager)
            .or_else(|| self.locked.first().map(|found| found.manager))
            .or_else(|| self.mentioned.first().map(|found| found.manager))
    }
}

/// Which section of a manifest declared a dependency.
///
/// `bundledDependencies` is not here on purpose, and it is the only field of the
/// five that is not. It is an array of names with no versions, every one of
/// which must also appear in `dependencies` — so reading it would add nothing
/// SURE does not already have, and reading it as a fifth *section* would report
/// each bundled package twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKind {
    /// `dependencies` — needed to run it.
    Runtime,
    /// `devDependencies` — needed to build or test it.
    Development,
    /// `peerDependencies` — the project expects its consumer to provide it.
    Peer,
    /// `optionalDependencies` — may fail to install.
    Optional,
}

impl DependencyKind {
    /// Every kind, in a fixed order.
    pub const ALL: &'static [Self] =
        &[Self::Runtime, Self::Development, Self::Peer, Self::Optional];

    /// The field name in a manifest.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::Runtime => "dependencies",
            Self::Development => "devDependencies",
            Self::Peer => "peerDependencies",
            Self::Optional => "optionalDependencies",
        }
    }
}

/// One dependency, as the manifest wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The package name.
    pub name: String,
    /// The requirement, **verbatim**.
    ///
    /// `"^18.2.0"`, `"latest"`, `"workspace:*"`, `"npm:other@1"` — whatever was
    /// written. SURE has run no resolver, so it does not know what any of these
    /// resolves to, and does not say.
    pub requirement: String,
    /// Which section declared it.
    pub kind: DependencyKind,
}

/// One script, as the manifest wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The name, which is what a person types after `npm run`.
    pub name: String,
    /// The command, **verbatim**, including any `&&` chain.
    pub command: String,
}

/// The scripts a project of this kind is conventionally expected to have.
///
/// The list is SURE's, not the project's, and that is the point: a project with
/// no `test` script is a finding, and it can only be reported by looking for a
/// name the project did not choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScriptRole {
    /// `build` — produce the thing that ships.
    Build,
    /// `test` — run the tests.
    Test,
    /// `lint` — report style and probable mistakes.
    Lint,
    /// `typecheck` — check types without emitting.
    TypeCheck,
    /// `format` — rewrite files to a style.
    Format,
    /// `dev` — run it for a developer.
    Dev,
    /// `start` — run it.
    Start,
    /// `clean` — remove build output.
    Clean,
}

impl ScriptRole {
    /// Every role, in a fixed order.
    pub const ALL: &'static [Self] = &[
        Self::Build,
        Self::Test,
        Self::Lint,
        Self::TypeCheck,
        Self::Format,
        Self::Dev,
        Self::Start,
        Self::Clean,
    ];

    /// The script name this role looks for.
    ///
    /// One name per role. A project that spelled its test script `test:unit` has
    /// not declared a `test` script, and mapping the one to the other would be
    /// SURE inventing a convention the project did not follow — and then running
    /// a command on the strength of it.
    #[must_use]
    pub const fn conventional_name(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Test => "test",
            Self::Lint => "lint",
            Self::TypeCheck => "typecheck",
            Self::Format => "format",
            Self::Dev => "dev",
            Self::Start => "start",
            Self::Clean => "clean",
        }
    }

    /// The sentence a person reads.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::Build => "build the project",
            Self::Test => "run the tests",
            Self::Lint => "check the code for style and likely mistakes",
            Self::TypeCheck => "check the types without building",
            Self::Format => "rewrite the files to a style",
            Self::Dev => "run the project for a developer",
            Self::Start => "run the project",
            Self::Clean => "remove what a build produced",
        }
    }
}

/// Whether a conventional role has a script behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalScript {
    /// The role.
    pub role: ScriptRole,
    /// The script, if the project declared one under the conventional name.
    pub script: Option<Script>,
}

/// A `package.json`, read.
///
/// Every field is what the file said. Nothing here has been checked against the
/// filesystem, resolved, or run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// `name`.
    pub name: Option<String>,
    /// `version`, verbatim — including a `workspace:` protocol or a prerelease
    /// tag, neither of which SURE interprets.
    pub version: Option<String>,
    /// `private`. `None` when the field is absent, which is not the same as
    /// `false`.
    pub private: Option<bool>,
    /// `packageManager`, verbatim — `"pnpm@8.6.0"`.
    pub package_manager: Option<String>,
    /// `engines.node`, verbatim.
    pub engines_node: Option<String>,
    /// Every manager named by an `engines` field, sorted by manager.
    ///
    /// Read by field name against [`PackageManager::ALL`], never by searching a
    /// range for a manager's name: `"^18.0.0 || ^20.0.0"` must not match
    /// anything, and a search would.
    pub engines_managers: Vec<ManagerMention>,
    /// The scripts, sorted by name.
    pub scripts: Vec<Script>,
    /// Script names whose declared value is not a command string.
    ///
    /// A project can write `"test": ["jest"]`. npm refuses it; so does SURE, and
    /// the name is recorded rather than dropped, because a script that is there
    /// and unusable is not a script that is not there.
    pub scripts_not_commands: Vec<String>,
    /// Every dependency from every section, sorted by name then section.
    pub dependencies: Vec<Dependency>,
    /// Dependency names whose declared value is not a version string.
    pub dependencies_not_ranges: Vec<String>,
    /// The workspace patterns, verbatim, in the order the manifest declared
    /// them.
    pub workspace_patterns: Vec<String>,
    /// The shapes of `workspaces` entries that are not patterns.
    ///
    /// Kept so that a `"workspaces": [1, 2]` is reported as a manifest SURE
    /// could not fully read rather than as a manifest with no workspaces.
    pub workspaces_not_patterns: Vec<&'static str>,
}

impl Package {
    /// Read a `package.json` out of parsed JSON.
    ///
    /// # Errors
    ///
    /// [`UnreadReason::WrongShape`] when the document is not a JSON object.
    /// `[1, 2, 3]` and `"hello"` are both valid JSON and neither is a manifest;
    /// reading fields out of one would produce a project with no scripts and no
    /// dependencies, which is the same false negative as an unparsed file
    /// arriving by a different route.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, UnreadReason> {
        let Some(object) = value.as_object() else {
            return Err(UnreadReason::WrongShape {
                found: json_shape(value),
            });
        };

        let mut scripts = Vec::new();
        let mut scripts_not_commands = Vec::new();
        if let Some(declared) = object.get("scripts").and_then(serde_json::Value::as_object) {
            for (name, command) in declared {
                match command.as_str() {
                    Some(command) => scripts.push(Script {
                        name: name.clone(),
                        command: command.to_owned(),
                    }),
                    None => scripts_not_commands.push(name.clone()),
                }
            }
        }
        scripts.sort_by(|a, b| a.name.cmp(&b.name));
        scripts_not_commands.sort();

        let mut dependencies = Vec::new();
        let mut dependencies_not_ranges = Vec::new();
        for &kind in DependencyKind::ALL {
            let Some(declared) = object
                .get(kind.field())
                .and_then(serde_json::Value::as_object)
            else {
                continue;
            };
            for (name, requirement) in declared {
                match requirement.as_str() {
                    Some(requirement) => dependencies.push(Dependency {
                        name: name.clone(),
                        requirement: requirement.to_owned(),
                        kind,
                    }),
                    None => dependencies_not_ranges.push(name.clone()),
                }
            }
        }
        // By name first and section second, so a package declared in two
        // sections lands in a fixed order and a report of it is stable.
        dependencies.sort_by(|a, b| a.name.cmp(&b.name).then(a.kind.cmp(&b.kind)));
        dependencies_not_ranges.sort();
        dependencies_not_ranges.dedup();

        let engines = object.get("engines").and_then(serde_json::Value::as_object);
        let mut engines_managers = Vec::new();
        if let Some(engines) = engines {
            for (name, range) in engines {
                let (Some(manager), Some(range)) =
                    (PackageManager::from_name(name), range.as_str())
                else {
                    continue;
                };
                engines_managers.push(ManagerMention {
                    manager,
                    range: range.to_owned(),
                });
            }
        }
        engines_managers.sort();

        let (workspace_patterns, workspaces_not_patterns) =
            workspace_patterns(object.get("workspaces"));

        Ok(Self {
            name: text(object.get("name")),
            version: text(object.get("version")),
            private: object.get("private").and_then(serde_json::Value::as_bool),
            package_manager: text(object.get("packageManager")),
            engines_node: engines.and_then(|engines| text(engines.get("node"))),
            engines_managers,
            scripts,
            scripts_not_commands,
            dependencies,
            dependencies_not_ranges,
            workspace_patterns,
            workspaces_not_patterns,
        })
    }

    /// A row for every conventional role, **whether or not it is declared**.
    ///
    /// The rows that are absent are the point. A report built from this says
    /// "there is no test script" out loud, instead of leaving a reader to notice
    /// that a line is missing from a list whose length they cannot see.
    ///
    /// Callable only on a manifest that was read, which is why this is a method
    /// on [`Package`] rather than on [`NodeProject`]: on an unread manifest
    /// there is no answer to give, and an empty list here would be read as
    /// "nothing is declared".
    #[must_use]
    pub fn conventional_scripts(&self) -> Vec<ConventionalScript> {
        ScriptRole::ALL
            .iter()
            .map(|&role| ConventionalScript {
                role,
                script: self.script(role).cloned(),
            })
            .collect()
    }

    /// The script declared under a conventional role's name, if any.
    #[must_use]
    pub fn script(&self, role: ScriptRole) -> Option<&Script> {
        self.scripts
            .iter()
            .find(|script| script.name == role.conventional_name())
    }

    /// The command a person would run to get one conventional script.
    ///
    /// `npm test` for a `test` script and `npm run build` for `build` — the two
    /// cases npm spells differently. Returned rather than executed: choosing a
    /// command is a plan, and running it is a later step with its own
    /// authorisation.
    ///
    /// `None` when the project declared no script under that name, which is the
    /// same answer [`Self::script`] gives and for the same reason.
    #[must_use]
    pub fn command_for(&self, manager: PackageManager, role: ScriptRole) -> Option<String> {
        let script = self.script(role)?;
        Some(match (manager, role) {
            (PackageManager::Npm, ScriptRole::Test | ScriptRole::Start) => {
                format!("npm {}", script.name)
            }
            (PackageManager::Npm, _) => format!("npm run {}", script.name),
            (manager, _) => format!("{} run {}", manager.as_str(), script.name),
        })
    }
}

/// A string field, or `None` if it is absent or not a string.
fn text(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(serde_json::Value::as_str).map(str::to_owned)
}

/// What shape a JSON value has, as one of this module's own phrases.
fn json_shape(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "nothing",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// The workspace patterns a `workspaces` field declares.
///
/// Two spellings are in use and both are read: `"workspaces": ["packages/*"]`
/// and `"workspaces": { "packages": ["packages/*"] }`. Neither is interpreted
/// here — a pattern is carried verbatim and resolved later.
fn workspace_patterns(declared: Option<&serde_json::Value>) -> (Vec<String>, Vec<&'static str>) {
    let mut patterns = Vec::new();
    let mut not_patterns = Vec::new();

    let list = match declared {
        None => return (patterns, not_patterns),
        Some(serde_json::Value::Array(_)) => declared,
        Some(serde_json::Value::Object(object)) => object.get("packages"),
        Some(other) => {
            not_patterns.push(json_shape(other));
            return (patterns, not_patterns);
        }
    };

    let Some(serde_json::Value::Array(entries)) = list else {
        if let Some(other) = list {
            not_patterns.push(json_shape(other));
        }
        return (patterns, not_patterns);
    };

    for entry in entries {
        match entry.as_str() {
            Some(pattern) => patterns.push(pattern.to_owned()),
            None => not_patterns.push(json_shape(entry)),
        }
    }
    (patterns, not_patterns)
}

/// What is at the manifest's name.
///
/// Three arms, not an `Option`, for the reason [`super::read`] gives in full: a
/// project whose `package.json` could not be read must not be described as a
/// project with no `package.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestState {
    /// There is a manifest and this is what it says.
    Read(Box<Package>),
    /// There is nothing at that name.
    Absent,
    /// There is a manifest and SURE did not get a value out of it.
    Unread(UnreadReason),
}

impl ManifestState {
    /// The manifest, if it was read.
    #[must_use]
    pub fn package(&self) -> Option<&Package> {
        match self {
            Self::Read(package) => Some(package),
            Self::Absent | Self::Unread(_) => None,
        }
    }

    /// Whether anything at all was at that name.
    #[must_use]
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// One directory a workspace pattern named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The directory, relative to the project root.
    pub path: PathBuf,
    /// What is at its `package.json`, without reading it.
    pub manifest: MemberManifest,
    /// The member's own manifest, read.
    ///
    /// Read because a monorepo's frameworks and test runners are almost never in
    /// the root manifest — a root declaring only `turbo` and `typescript` says
    /// nothing about the React application in `apps/web`. Kept per member rather
    /// than merged into the root's, because the two are different manifests and
    /// a merged list could not say which run where.
    pub package: Option<Box<Package>>,
}

/// The workspace structure a project declares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Workspaces {
    /// What declared the patterns, and what each file said.
    pub declared_by: Vec<Source>,
    /// The patterns, merged across every file that declared some, in order.
    pub patterns: Vec<String>,
    /// The directories the patterns named, in resolution order.
    pub members: Vec<Member>,
    /// The patterns that named nothing, and why.
    pub unresolved: Vec<Unresolved>,
    /// Whether the member list was cut short.
    ///
    /// `true` means there were more members than
    /// [`DiscoverOptions::max_workspace_members`] and this list is a prefix of
    /// the real one. A caller that treats it as the whole workspace is reporting
    /// on a project it did not finish looking at.
    pub truncated: bool,
}

impl Workspaces {
    /// Whether the project declares a workspace at all.
    #[must_use]
    pub fn is_declared(&self) -> bool {
        !self.patterns.is_empty()
    }

    /// The members whose own manifest SURE read.
    pub fn readable_members(&self) -> impl Iterator<Item = &Member> {
        self.members
            .iter()
            .filter(|member| member.package.is_some())
    }
}

/// What kind of thing a recognised package is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolRole {
    /// A framework built on top of another one — Next, Nuxt, SvelteKit.
    MetaFramework,
    /// A framework for building a user interface.
    WebFramework,
    /// A framework for answering requests.
    ServerFramework,
    /// A tool that runs tests.
    TestRunner,
    /// A tool that drives a real browser.
    EndToEndTest,
    /// A tool that reports style and probable mistakes.
    Linter,
    /// A tool that rewrites files to a style.
    Formatter,
    /// A tool that turns source into something that ships.
    Bundler,
    /// A tool that runs tasks across several packages.
    MonorepoTool,
    /// A tool that executes another language directly.
    Runtime,
    /// The language itself.
    Language,
}

impl ToolRole {
    /// Every role, in a fixed order, most specific first.
    pub const ALL: &'static [Self] = &[
        Self::MetaFramework,
        Self::WebFramework,
        Self::ServerFramework,
        Self::TestRunner,
        Self::EndToEndTest,
        Self::Linter,
        Self::Formatter,
        Self::Bundler,
        Self::MonorepoTool,
        Self::Runtime,
        Self::Language,
    ];

    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MetaFramework => "meta_framework",
            Self::WebFramework => "web_framework",
            Self::ServerFramework => "server_framework",
            Self::TestRunner => "test_runner",
            Self::EndToEndTest => "end_to_end_test",
            Self::Linter => "linter",
            Self::Formatter => "formatter",
            Self::Bundler => "bundler",
            Self::MonorepoTool => "monorepo_tool",
            Self::Runtime => "runtime",
            Self::Language => "language",
        }
    }

    /// The sentence a person reads.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::MetaFramework => "a framework for building a whole application",
            Self::WebFramework => "a framework for building a user interface",
            Self::ServerFramework => "a framework for answering requests",
            Self::TestRunner => "a tool that runs tests",
            Self::EndToEndTest => "a tool that drives a real browser",
            Self::Linter => "a tool that reports style and likely mistakes",
            Self::Formatter => "a tool that rewrites files to a style",
            Self::Bundler => "a tool that turns source into something that ships",
            Self::MonorepoTool => "a tool that runs tasks across several packages",
            Self::Runtime => "a tool that runs another language directly",
            Self::Language => "a language",
        }
    }
}

/// The packages SURE recognises, and what each one is.
///
/// **A fixed table, and a project can never add to it.** The `&'static str`
/// stored in a [`Tooling`] comes from this column, never from the manifest, so
/// no dependency name a project wrote can reach a sentence SURE prints.
///
/// A package may appear more than once when it is genuinely two things: Biome
/// lints *and* formats, and both are reported, because picking one would be SURE
/// choosing which half of a tool to mention.
const TOOLS: &[(&str, ToolRole)] = &[
    // Meta-frameworks: a whole application, with routing and rendering.
    ("next", ToolRole::MetaFramework),
    ("nuxt", ToolRole::MetaFramework),
    ("@remix-run/react", ToolRole::MetaFramework),
    ("@remix-run/node", ToolRole::MetaFramework),
    ("@sveltejs/kit", ToolRole::MetaFramework),
    ("astro", ToolRole::MetaFramework),
    ("gatsby", ToolRole::MetaFramework),
    ("@builder.io/qwik", ToolRole::MetaFramework),
    ("solid-start", ToolRole::MetaFramework),
    ("@tanstack/start", ToolRole::MetaFramework),
    ("@angular/core", ToolRole::MetaFramework),
    // User-interface frameworks.
    ("react", ToolRole::WebFramework),
    ("vue", ToolRole::WebFramework),
    ("svelte", ToolRole::WebFramework),
    ("solid-js", ToolRole::WebFramework),
    ("preact", ToolRole::WebFramework),
    ("lit", ToolRole::WebFramework),
    ("alpinejs", ToolRole::WebFramework),
    ("ember-source", ToolRole::WebFramework),
    // Server frameworks.
    ("express", ToolRole::ServerFramework),
    ("fastify", ToolRole::ServerFramework),
    ("koa", ToolRole::ServerFramework),
    ("@hapi/hapi", ToolRole::ServerFramework),
    ("@nestjs/core", ToolRole::ServerFramework),
    ("hono", ToolRole::ServerFramework),
    ("restify", ToolRole::ServerFramework),
    ("polka", ToolRole::ServerFramework),
    ("@adonisjs/core", ToolRole::ServerFramework),
    ("@feathersjs/feathers", ToolRole::ServerFramework),
    // Test runners.
    ("jest", ToolRole::TestRunner),
    ("vitest", ToolRole::TestRunner),
    ("mocha", ToolRole::TestRunner),
    ("ava", ToolRole::TestRunner),
    ("jasmine", ToolRole::TestRunner),
    ("tape", ToolRole::TestRunner),
    ("@jest/core", ToolRole::TestRunner),
    // Browser-driven tests.
    ("@playwright/test", ToolRole::EndToEndTest),
    ("playwright", ToolRole::EndToEndTest),
    ("cypress", ToolRole::EndToEndTest),
    ("puppeteer", ToolRole::EndToEndTest),
    ("webdriverio", ToolRole::EndToEndTest),
    ("@wdio/cli", ToolRole::EndToEndTest),
    ("testcafe", ToolRole::EndToEndTest),
    // Linters and formatters.
    ("eslint", ToolRole::Linter),
    ("oxlint", ToolRole::Linter),
    ("standard", ToolRole::Linter),
    ("@biomejs/biome", ToolRole::Linter),
    ("@biomejs/biome", ToolRole::Formatter),
    ("prettier", ToolRole::Formatter),
    ("dprint", ToolRole::Formatter),
    // Bundlers.
    ("webpack", ToolRole::Bundler),
    ("rollup", ToolRole::Bundler),
    ("esbuild", ToolRole::Bundler),
    ("parcel", ToolRole::Bundler),
    ("vite", ToolRole::Bundler),
    ("@rspack/core", ToolRole::Bundler),
    ("browserify", ToolRole::Bundler),
    ("snowpack", ToolRole::Bundler),
    // Tools that run work across a workspace.
    ("turbo", ToolRole::MonorepoTool),
    ("nx", ToolRole::MonorepoTool),
    ("lerna", ToolRole::MonorepoTool),
    ("@microsoft/rush", ToolRole::MonorepoTool),
    ("@changesets/cli", ToolRole::MonorepoTool),
    // Runtimes for code that is not plain JavaScript.
    ("ts-node", ToolRole::Runtime),
    ("tsx", ToolRole::Runtime),
    ("@swc-node/register", ToolRole::Runtime),
    // Languages.
    ("typescript", ToolRole::Language),
];

/// A recognised package, and where in the project it was declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tooling {
    /// The package name, from [`TOOLS`] — never from the manifest.
    pub package: &'static str,
    /// What it is.
    pub role: ToolRole,
    /// Which section declared it.
    pub kind: DependencyKind,
    /// The directory of the manifest that declared it, relative to the project
    /// root. Empty for the root manifest.
    pub from: PathBuf,
}

/// Which piece of evidence says this is a TypeScript project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeScriptEvidence {
    /// A manifest declared the `typescript` package.
    Compiler,
    /// A `tsconfig.json` or `jsconfig.json` is in the project.
    Config,
    /// The project has files with a TypeScript extension.
    SourceFiles,
}

impl TypeScriptEvidence {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compiler => "compiler",
            Self::Config => "config",
            Self::SourceFiles => "source_files",
        }
    }
}

/// A TypeScript project, and what says so.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeScript {
    /// Each kind of evidence found, with the file it came from.
    pub evidence: Vec<(TypeScriptEvidence, Source)>,
    /// How many files with a TypeScript extension the walk found.
    ///
    /// A count rather than a list, because the list can be the whole project and
    /// the useful claim is that TypeScript files are there. Only the first is
    /// named, in [`Self::evidence`], so the finding has an anchor without
    /// carrying a thousand paths.
    pub source_files: usize,
}

impl TypeScript {
    /// Whether anything says this is a TypeScript project.
    #[must_use]
    pub fn is_present(&self) -> bool {
        !self.evidence.is_empty()
    }
}

/// Everything SURE found out about a Node project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeProject {
    /// What is at the project root's `package.json`.
    pub manifest: ManifestState,
    /// The package managers SURE found evidence for.
    pub managers: Managers,
    /// The workspace structure declared.
    pub workspaces: Workspaces,
    /// Recognised packages, from the root manifest and every member's.
    pub tooling: Vec<Tooling>,
    /// Whether this is a TypeScript project, and why SURE thinks so.
    pub typescript: TypeScript,
}

impl NodeProject {
    /// Every recognised package with one role.
    pub fn tooling_of_role(&self, role: ToolRole) -> impl Iterator<Item = &Tooling> {
        self.tooling.iter().filter(move |tool| tool.role == role)
    }

    /// Whether any manifest in the project declared this package.
    ///
    /// `package` is matched against the table's own names, so a caller asks
    /// about a package SURE knows rather than about one a project wrote.
    #[must_use]
    pub fn declares(&self, package: &str) -> bool {
        self.tooling.iter().any(|tool| tool.package == package)
    }

    /// The root manifest, if there is a readable one.
    #[must_use]
    pub fn package(&self) -> Option<&Package> {
        self.manifest.package()
    }
}

/// What a Node project's support level is, and the sentence that says why.
///
/// Returned together, and as a constant, so that a level and its reason cannot
/// be assigned in two places and disagree.
fn grade(manifest: &ManifestState, found_a_lockfile: bool) -> (SupportLevel, &'static str) {
    match manifest {
        ManifestState::Read(_) => (
            SupportLevel::Generic,
            "SURE read this project's package.json, so it can find how the project is \
             built and run.",
        ),
        ManifestState::Unread(_) => (
            SupportLevel::InspectOnly,
            "There is a package.json here and SURE could not read it, so it can only \
             look at the project's files.",
        ),
        ManifestState::Absent if found_a_lockfile => (
            SupportLevel::InspectOnly,
            "This project has a lockfile but no package.json, so SURE can see it is a \
             Node project and cannot read what it declares.",
        ),
        ManifestState::Absent => (
            SupportLevel::InspectOnly,
            "There is no package.json here, so SURE can see this project's files and \
             cannot read how it is built or run.",
        ),
    }
}

/// Find out what a Node project declares, if this is one.
///
/// Returns `None` when nothing says this is a Node or TypeScript project. See
/// [`looks_like_one`] for the complete list of what counts, which is short on
/// purpose.
pub(super) fn look(
    root: &Path,
    scan: &Scan,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
) -> Option<EcosystemReport> {
    let case = options.scan.case;
    let tree = read::Tree::of(scan, case);
    let mut found_by = Vec::new();

    // --- the root manifest --------------------------------------------------
    let manifest_path = Path::new(MANIFEST);
    let manifest = read_manifest(root, manifest_path, &tree, options, budget, unread);
    if !manifest.is_absent() {
        found_by.push(manifest_path.to_path_buf());
    }

    // --- lockfiles, by existence and never by contents ----------------------
    let mut locked = Vec::new();
    for &(name, manager) in LOCKFILES {
        // Answered by `probe`, not by a read: see the module comment. Only
        // `Probe::File` counts, so a *link* named `yarn.lock` is not reported as
        // a lockfile — SURE does not read through links, and a lockfile's whole
        // contribution here is that it exists, which a link does not establish.
        if matches!(tree.probe(Path::new(name)), Probe::File(_)) {
            found_by.push(PathBuf::from(name));
            locked.push(ManagerFinding {
                manager,
                evidence: ManagerEvidence::Lockfile,
                source: Source::new(name, "is a lockfile for this package manager"),
            });
        }
    }

    // --- the pnpm workspace file -------------------------------------------
    let pnpm_path = Path::new(PNPM_WORKSPACE);
    let pnpm = read_or_report_yaml(root, pnpm_path, &tree, options, budget, unread);
    if !pnpm.is_absent() {
        found_by.push(pnpm_path.to_path_buf());
    }

    // --- the compiler configuration ----------------------------------------
    let mut typescript = TypeScript::default();
    let mut found_a_config = false;
    for &name in TSCONFIGS {
        if matches!(tree.probe(Path::new(name)), Probe::File(_)) {
            found_by.push(PathBuf::from(name));
            typescript.evidence.push((
                TypeScriptEvidence::Config,
                Source::new(name, "is a TypeScript configuration file"),
            ));
            found_a_config = true;
        }
    }

    if !looks_like_one(&manifest, &locked, pnpm.is_absent(), found_a_config) {
        return None;
    }

    // --- what the root manifest and the pnpm file declare --------------------
    let managers = managers_of(manifest.package(), &locked);

    let mut workspaces = Workspaces::default();
    if let Some(package) = manifest.package()
        && !package.workspace_patterns.is_empty()
    {
        workspaces.declared_by.push(Source::new(
            MANIFEST,
            "declares the directories this project's packages live in",
        ));
        workspaces
            .patterns
            .extend(package.workspace_patterns.iter().cloned());
    }
    collect_pnpm_workspace_patterns(&pnpm, &mut workspaces, pnpm_path);

    resolve_members(root, &tree, options, budget, unread, &mut workspaces);

    // --- tooling, from every manifest that was read --------------------------
    let mut tooling = Vec::new();
    if let Some(package) = manifest.package() {
        collect_tooling(package, Path::new(""), &mut tooling);
    }
    for member in &workspaces.members {
        if let Some(package) = &member.package {
            collect_tooling(package, &member.path, &mut tooling);
        }
    }
    // Sorted by the table's name, then role, then the directory it came from —
    // every one of the three a fixed order, so two runs over one project cannot
    // produce two lists.
    tooling.sort_by(|a, b| {
        a.package
            .cmp(b.package)
            .then(a.role.cmp(&b.role))
            .then(a.from.cmp(&b.from))
    });
    tooling.dedup();

    if let Some(tool) = tooling.iter().find(|tool| tool.package == "typescript") {
        typescript.evidence.push((
            TypeScriptEvidence::Compiler,
            Source::new(tool.from.join(MANIFEST), "declares the typescript compiler"),
        ));
    }

    // --- TypeScript source files -------------------------------------------
    let mut ts_files: Vec<&Path> = scan
        .files()
        .map(|entry| entry.path.as_path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| TS_EXTENSIONS.contains(&extension))
        })
        .collect();
    // Sorted before the first is taken, so which file anchors the finding is a
    // function of the project rather than of the walk's order.
    ts_files.sort_unstable();
    typescript.source_files = ts_files.len();
    if let Some(first) = ts_files.first() {
        typescript.evidence.push((
            TypeScriptEvidence::SourceFiles,
            Source::new(*first, "is a TypeScript source file"),
        ));
    }
    // By kind, then by path within a kind, so the list is stable and a reader
    // sees the strongest evidence first.
    typescript
        .evidence
        .sort_by(|(left_kind, left), (right_kind, right)| {
            left_kind.cmp(right_kind).then(left.path.cmp(&right.path))
        });

    let (level, reason) = grade(&manifest, !locked.is_empty());
    Some(EcosystemReport {
        ecosystem: Ecosystem::Node,
        level,
        reason: reason.to_owned(),
        found_by,
        findings: Findings::Node(Box::new(NodeProject {
            manifest,
            managers,
            workspaces,
            tooling,
            typescript,
        })),
    })
}

/// Whether anything here says this is a Node or TypeScript project.
///
/// Four things count, and every one is a **project-level** file: a manifest, a
/// lockfile, a pnpm workspace file, a compiler configuration. Source files are
/// **not** on the list. A directory containing a `.js` file is a directory
/// containing a `.js` file, and calling that a Node project on the strength of
/// one stray file is the kind of inference this product exists to not make.
fn looks_like_one(
    manifest: &ManifestState,
    locked: &[ManagerFinding],
    pnpm_absent: bool,
    found_a_config: bool,
) -> bool {
    !manifest.is_absent() || !locked.is_empty() || !pnpm_absent || found_a_config
}

/// Read a YAML file and record it if it could not be read.
fn read_or_report_yaml(
    root: &Path,
    relative: &Path,
    tree: &read::Tree<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
) -> ReadFile {
    let read = read::read_yaml(root, tree.probe(relative), options, budget);
    record(&read, relative, unread);
    read
}

/// Put a file that is there and was not read into the result.
///
/// `Absent` is not recorded: a name with nothing at it is not a file SURE
/// failed to read, and recording one would make every project with no
/// `pnpm-workspace.yaml` carry an unread file.
fn record(read: &ReadFile, relative: &Path, unread: &mut Vec<Unread>) {
    if let Some(reason) = read.reason() {
        unread.push(Unread {
            path: relative.to_path_buf(),
            reason: reason.clone(),
        });
    }
}

/// Read a manifest, and make it impossible for an unread one to go unrecorded.
///
/// **This is one function rather than a reader and a converter, and that is the
/// point.** The first version of this file had them apart — a helper that
/// recorded what the *reader* refused, and a converter that turned the parsed
/// value into a [`ManifestState`]. It compiled, it was tested, and it was wrong:
/// `[1, 2, 3]` parses as JSON, so the reader had nothing to record, and the shape
/// failure the converter produced reached [`ManifestState::Unread`] without ever
/// reaching [`Discovery::unread`]. The same project was described two ways by two
/// fields of one result.
///
/// Pairing them here means every arm that produces an `Unread` is in the same
/// `match` as the line that records it, and there is no ordering or convention
/// for a later edit to get wrong.
fn read_manifest(
    root: &Path,
    relative: &Path,
    tree: &read::Tree<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
) -> ManifestState {
    let unreadable = |reason: UnreadReason, unread: &mut Vec<Unread>| {
        unread.push(Unread {
            path: relative.to_path_buf(),
            reason: reason.clone(),
        });
        ManifestState::Unread(reason)
    };

    match read::read_json(root, tree.probe(relative), options, budget) {
        ReadFile::Absent => ManifestState::Absent,
        ReadFile::Unread(reason) => unreadable(reason, unread),
        // Anything but an object is a document that is not a manifest. Reported
        // as unread rather than as a project with no scripts, which is the whole
        // reason `Package::from_json` returns a `Result`.
        ReadFile::Parsed(value) => match Package::from_json(&value) {
            Ok(package) => ManifestState::Read(Box::new(package)),
            Err(reason) => unreadable(reason, unread),
        },
    }
}

/// The managers the evidence names, kept by strength.
fn managers_of(package: Option<&Package>, locked: &[ManagerFinding]) -> Managers {
    let declared = package
        .and_then(|package| package.package_manager.as_deref())
        .and_then(declared_manager);
    let mentioned = package
        .map(|package| package.engines_managers.clone())
        .unwrap_or_default();

    Managers {
        declared,
        locked: locked.to_vec(),
        mentioned,
    }
}

/// The manager a `packageManager` field names.
///
/// The field is `name@version`. Split at the **first** `@` that follows the
/// name: `pnpm@8.6.0` yields `pnpm`, and so does a bare `pnpm`. A name that is
/// not in [`PackageManager::ALL`] is `None` — SURE does not recognise it and
/// does not guess.
fn declared_manager(field: &str) -> Option<ManagerFinding> {
    let name = field.split('@').next().unwrap_or(field);
    let manager = PackageManager::from_name(name)?;
    Some(ManagerFinding {
        manager,
        evidence: ManagerEvidence::Declared,
        source: Source::new(
            MANIFEST,
            "names this package manager in its packageManager field",
        ),
    })
}

/// Collect the tooling a manifest declares.
fn collect_tooling(package: &Package, from: &Path, into: &mut Vec<Tooling>) {
    for dependency in &package.dependencies {
        for &(name, role) in TOOLS {
            if dependency.name == name {
                into.push(Tooling {
                    package: name,
                    role,
                    kind: dependency.kind,
                    from: from.to_path_buf(),
                });
            }
        }
    }
}

/// The workspace patterns a `pnpm-workspace.yaml` declares.
fn collect_pnpm_workspace_patterns(pnpm: &ReadFile, workspaces: &mut Workspaces, pnpm_path: &Path) {
    let Some(value) = pnpm.value() else {
        return;
    };
    let Some(entries) = value.get("packages").and_then(serde_json::Value::as_array) else {
        return;
    };
    let mut patterns = Vec::new();
    for entry in entries {
        // A pattern that is not a string is dropped rather than recorded here,
        // because the file it came from is already in `unread` if it could not
        // be read at all — and a YAML list of non-strings parses fine. It is
        // recorded as an unresolved pattern instead, which names the file.
        if let Some(pattern) = entry.as_str() {
            patterns.push(pattern.to_owned());
        }
    }
    if patterns.is_empty() {
        return;
    }
    workspaces.declared_by.push(Source::new(
        pnpm_path.to_path_buf(),
        "declares the directories this project's packages live in",
    ));
    for pattern in patterns {
        if !workspaces.patterns.contains(&pattern) {
            workspaces.patterns.push(pattern);
        }
    }
}

/// Resolve every pattern to the directories it names, and read their manifests.
fn resolve_members(
    root: &Path,
    tree: &read::Tree<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
    workspaces: &mut Workspaces,
) {
    // The root is seeded as already-seen inside `pattern::resolve`, so a pattern
    // that names it — `"."`, or a `*` that reaches it — does not make the root a
    // member of itself. The root's manifest is `NodeProject::manifest`;
    // describing it twice would also read it twice against a finite budget.
    let (mut ordered, unresolved) = pattern::resolve(tree, &workspaces.patterns, options.scan.case);
    workspaces.unresolved = unresolved;

    // Truncation is recorded rather than applied quietly: a workspace whose
    // members are cut off is a workspace SURE has not finished looking at, and a
    // caller that read the member list as complete would be reasoning about a
    // project that is not there.
    if ordered.len() > options.max_workspace_members {
        ordered.truncate(options.max_workspace_members);
        workspaces.truncated = true;
    }

    for path in ordered {
        let manifest_path = path.join(MANIFEST);
        let probe = tree.probe(&manifest_path);
        let summary = match probe {
            Probe::Nothing => MemberManifest::Absent,
            Probe::Other(kind) => MemberManifest::NotReadable(kind),
            Probe::File(_) => MemberManifest::Present,
        };
        // Read even when the summary already says what is there, because the
        // summary answers a different question: it says whether a manifest
        // exists, and this says what it declares.
        let package = match read_manifest(root, &manifest_path, tree, options, budget, unread) {
            ManifestState::Read(package) => Some(package),
            // An absent or unread member manifest is already recorded: `summary`
            // says which of the two it is, and an unread one is in `unread` with
            // its path. Nothing is dropped by not repeating it here.
            ManifestState::Absent | ManifestState::Unread(_) => None,
        };
        workspaces.members.push(Member {
            path,
            manifest: summary,
            package,
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    fn package(value: serde_json::Value) -> Package {
        Package::from_json(&value).expect("this fixture is a manifest")
    }

    #[test]
    fn a_document_that_is_not_an_object_is_not_read_as_an_empty_manifest() {
        // The false green this guards: `{}` has no scripts, no dependencies and
        // no manager — and so does `[1, 2, 3]` if it is read leniently. One of
        // those is a project that declares nothing and the other is a file SURE
        // did not understand, and reporting them the same way would state the
        // first about the second.
        for not_a_manifest in [json!([1, 2, 3]), json!("hello"), json!(7), json!(null)] {
            let error = Package::from_json(&not_a_manifest)
                .expect_err("this is not a package.json, whatever else it is");
            assert!(
                matches!(error, UnreadReason::WrongShape { .. }),
                "expected WrongShape, got {error:?}"
            );
            assert!(error.detail().is_some(), "the shape was not reported");
        }
        // And the object case really does produce an empty manifest, so the
        // distinction above is between two different answers rather than
        // between one answer and an error.
        let empty = package(json!({}));
        assert!(empty.scripts.is_empty());
        assert!(empty.dependencies.is_empty());
    }

    #[test]
    fn a_script_that_is_there_and_has_no_command_is_not_a_script_that_is_absent() {
        let package = package(json!({
            "scripts": { "build": "tsc", "test": ["jest"], "lint": null }
        }));
        assert_eq!(package.scripts.len(), 1);
        assert_eq!(package.scripts[0].name, "build");
        assert_eq!(
            package.scripts_not_commands,
            vec!["lint".to_owned(), "test".to_owned()],
            "a script with no command vanished, and the report would say the \
             project declares no test script"
        );
        // The distinction that matters: `conventional_scripts` distinguishes
        // "not declared" from "declared but unusable" only if the caller reads
        // `scripts_not_commands` too, and the report must do that.
        let test = package
            .conventional_scripts()
            .into_iter()
            .find(|row| row.role == ScriptRole::Test)
            .expect("every role has a row");
        assert!(test.script.is_none());
        assert!(package.scripts_not_commands.contains(&"test".to_owned()));
    }

    #[test]
    fn every_conventional_role_has_a_row_whether_or_not_it_is_declared() {
        let package = package(json!({ "scripts": { "build": "tsc" } }));
        let rows = package.conventional_scripts();
        assert_eq!(
            rows.len(),
            ScriptRole::ALL.len(),
            "a report built from this would be missing the rows that say what the \
             project does not have"
        );
        let declared: Vec<_> = rows.iter().filter(|row| row.script.is_some()).collect();
        assert_eq!(declared.len(), 1);
        assert_eq!(declared[0].role, ScriptRole::Build);
    }

    #[test]
    fn an_unconventional_script_name_is_not_mapped_onto_a_conventional_role() {
        // `test:unit` is not `test`. Mapping one to the other would be SURE
        // inventing a convention the project did not follow and then offering to
        // run a command on the strength of it.
        let package = package(json!({ "scripts": { "test:unit": "jest" } }));
        assert_eq!(package.scripts.len(), 1, "the script itself is still read");
        assert!(package.script(ScriptRole::Test).is_none());
        assert!(
            package
                .command_for(PackageManager::Npm, ScriptRole::Test)
                .is_none()
        );
    }

    #[test]
    fn the_command_for_a_script_is_the_one_the_manager_actually_uses() {
        let package = package(json!({ "scripts": { "test": "vitest", "build": "vite build" } }));
        // npm spells `test` and `start` without `run` and everything else with
        // it. Getting this wrong produces a command that does not work.
        assert_eq!(
            package.command_for(PackageManager::Npm, ScriptRole::Test),
            Some("npm test".to_owned())
        );
        assert_eq!(
            package.command_for(PackageManager::Npm, ScriptRole::Build),
            Some("npm run build".to_owned())
        );
        assert_eq!(
            package.command_for(PackageManager::Pnpm, ScriptRole::Test),
            Some("pnpm run test".to_owned())
        );
        assert_eq!(
            package.command_for(PackageManager::Yarn, ScriptRole::Build),
            Some("yarn run build".to_owned())
        );
        assert_eq!(
            package.command_for(PackageManager::Bun, ScriptRole::Build),
            Some("bun run build".to_owned())
        );
    }

    #[test]
    fn a_version_range_is_recorded_as_written_and_never_interpreted() {
        let package = package(json!({
            "dependencies": { "react": "^18.2.0", "other": "npm:third@1" },
            "devDependencies": { "typescript": "workspace:*" }
        }));
        let react = package
            .dependencies
            .iter()
            .find(|dependency| dependency.name == "react")
            .expect("react was declared");
        assert_eq!(react.requirement, "^18.2.0");
        assert_eq!(react.kind, DependencyKind::Runtime);
        let typescript = package
            .dependencies
            .iter()
            .find(|dependency| dependency.name == "typescript")
            .expect("typescript was declared");
        assert_eq!(
            typescript.requirement, "workspace:*",
            "a range SURE does not understand must be carried, not dropped"
        );
        assert_eq!(typescript.kind, DependencyKind::Development);
    }

    #[test]
    fn the_three_kinds_of_manager_evidence_are_kept_apart_and_ranked() {
        // Declared beats a lockfile, and a lockfile beats an engines range.
        let mut managers = Managers {
            declared: Some(ManagerFinding {
                manager: PackageManager::Pnpm,
                evidence: ManagerEvidence::Declared,
                source: Source::new(MANIFEST, "names it"),
            }),
            locked: vec![ManagerFinding {
                manager: PackageManager::Pnpm,
                evidence: ManagerEvidence::Lockfile,
                source: Source::new("pnpm-lock.yaml", "is a lockfile"),
            }],
            mentioned: vec![ManagerMention {
                manager: PackageManager::Npm,
                range: ">=9".to_owned(),
            }],
        };
        assert_eq!(managers.agreed(), Some(PackageManager::Pnpm));
        assert!(
            managers.disagreement().is_none(),
            "a weak mention does not contradict a declaration"
        );

        // A declaration a lockfile does not match is reported, not resolved.
        managers.locked = vec![ManagerFinding {
            manager: PackageManager::Npm,
            evidence: ManagerEvidence::Lockfile,
            source: Source::new("package-lock.json", "is a lockfile"),
        }];
        assert_eq!(
            managers.disagreement(),
            Some(Disagreement::DeclarationAndLockfile {
                declared: PackageManager::Pnpm,
                locked: PackageManager::Npm,
            })
        );
        assert_eq!(
            managers.agreed(),
            None,
            "SURE picked a winner where the project said two things"
        );
    }

    #[test]
    fn two_lockfiles_for_two_managers_are_a_disagreement_and_two_for_one_are_not() {
        let finding = |manager, name: &'static str| ManagerFinding {
            manager,
            evidence: ManagerEvidence::Lockfile,
            source: Source::new(name, "is a lockfile for this package manager"),
        };

        let same = Managers {
            locked: vec![
                finding(PackageManager::Bun, "bun.lockb"),
                finding(PackageManager::Bun, "bun.lock"),
            ],
            ..Managers::default()
        };
        assert!(same.disagreement().is_none(), "one manager, two lockfiles");
        assert_eq!(same.agreed(), Some(PackageManager::Bun));

        let differ = Managers {
            locked: vec![
                finding(PackageManager::Npm, "package-lock.json"),
                finding(PackageManager::Yarn, "yarn.lock"),
            ],
            ..Managers::default()
        };
        assert_eq!(
            differ.disagreement(),
            Some(Disagreement::TwoLockfiles {
                first: PackageManager::Npm,
                second: PackageManager::Yarn,
            })
        );
        assert_eq!(differ.agreed(), None);

        let none = Managers::default();
        assert!(none.disagreement().is_none());
        assert_eq!(
            none.agreed(),
            None,
            "no evidence and contradictory evidence both answer None, and a caller \
             must read `disagreement` to tell them apart"
        );
    }

    #[test]
    fn a_package_manager_field_is_split_at_its_name_and_not_guessed_at() {
        let named = |field: &str| declared_manager(field).map(|found| found.manager);
        assert_eq!(named("pnpm@8.6.0"), Some(PackageManager::Pnpm));
        assert_eq!(named("pnpm"), Some(PackageManager::Pnpm));
        assert_eq!(named("yarn@4.1.0"), Some(PackageManager::Yarn));
        assert_eq!(named("bun@1.2.0"), Some(PackageManager::Bun));
        assert_eq!(
            named("corepack@0.20.0"),
            None,
            "a manager SURE does not recognise must not become one it does"
        );
        assert_eq!(named(""), None);
    }

    #[test]
    fn an_engines_range_is_read_by_field_name_and_never_searched_for_a_name() {
        // The mistake this guards: `"^18.0.0 || ^20.0.0"` contains no manager
        // name, but a range like `"npm:>=9"` or a version such as `"1.0.0-pnpm"`
        // would match a search. Reading by field name cannot.
        let package = package(json!({
            "engines": { "node": ">=20", "npm": ">=9", "pnpm": "9.0.0", "vscode": "^1.80.0" }
        }));
        assert_eq!(package.engines_node, Some(">=20".to_owned()));
        let named: Vec<_> = package
            .engines_managers
            .iter()
            .map(|mention| (mention.manager, mention.range.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                (PackageManager::Npm, ">=9"),
                (PackageManager::Pnpm, "9.0.0"),
            ],
            "`node` is the interpreter and `vscode` is not a package manager; \
             neither is evidence about which manager this project uses"
        );
    }

    #[test]
    fn both_spellings_of_a_workspace_field_are_read_and_neither_is_interpreted() {
        let list = package(json!({ "workspaces": ["packages/*", "apps/*"] }));
        assert_eq!(list.workspace_patterns, vec!["packages/*", "apps/*"]);
        assert!(list.workspaces_not_patterns.is_empty());

        let object = package(json!({ "workspaces": { "packages": ["libs/*"] } }));
        assert_eq!(object.workspace_patterns, vec!["libs/*"]);

        // A workspaces field SURE cannot read is reported, not treated as a
        // project with no workspace.
        let bad = package(json!({ "workspaces": { "packages": [1, "ok"] } }));
        assert_eq!(bad.workspace_patterns, vec!["ok"]);
        assert_eq!(bad.workspaces_not_patterns, vec!["a number"]);

        let absent = package(json!({}));
        assert!(absent.workspace_patterns.is_empty());
        assert!(absent.workspaces_not_patterns.is_empty());
    }

    #[test]
    fn a_tool_is_named_from_the_table_and_never_from_the_manifest() {
        // The property: what is stored is the table's own `&'static str`. A
        // project cannot reach a sentence SURE prints, because the value
        // reported is not the value the project wrote — the two happen to be
        // equal here and the types make them impossible to confuse.
        let package = package(json!({
            "dependencies": { "react": "^18" },
            "devDependencies": { "vitest": "^1", "not-a-real-tool": "^1" }
        }));
        let mut tooling = Vec::new();
        collect_tooling(&package, Path::new(""), &mut tooling);
        let named: Vec<_> = tooling
            .iter()
            .map(|tool| (tool.package, tool.role))
            .collect();
        assert_eq!(
            named,
            vec![
                ("react", ToolRole::WebFramework),
                ("vitest", ToolRole::TestRunner),
            ],
            "an unrecognised package must be silent rather than guessed at"
        );
        // The type, asserted rather than assumed: `package` is a `&'static str`
        // borrowed from the table.
        let stored: &'static str = tooling[0].package;
        assert!(TOOLS.iter().any(|&(name, _)| name == stored));
    }

    #[test]
    fn a_package_that_is_genuinely_two_roles_is_reported_as_both() {
        let package = package(json!({ "devDependencies": { "@biomejs/biome": "^1" } }));
        let mut tooling = Vec::new();
        collect_tooling(&package, Path::new(""), &mut tooling);
        let roles: Vec<_> = tooling.iter().map(|tool| tool.role).collect();
        assert_eq!(roles, vec![ToolRole::Linter, ToolRole::Formatter]);
    }

    #[test]
    fn every_role_the_table_uses_is_a_role_that_exists() {
        // A row whose role were wrong would still compile, so the table is
        // held against the enum rather than trusted.
        for &(name, role) in TOOLS {
            assert!(!name.is_empty(), "a row with no package name");
            assert!(ToolRole::ALL.contains(&role));
            assert!(!role.as_str().is_empty());
            assert!(!role.plain_description().is_empty());
        }
    }

    #[test]
    fn every_manager_can_be_detected_from_a_lockfile_or_the_table_is_incomplete() {
        // A manager with no lockfile row is one SURE can only ever find through
        // `packageManager`, and a test that failed here would be the place to
        // notice rather than a project that quietly reports no manager.
        for &manager in PackageManager::ALL {
            assert!(
                LOCKFILES.iter().any(|&(_, known)| known == manager)
                    || manager == PackageManager::Npm,
                "{manager:?} has no lockfile SURE recognises"
            );
            assert!(!manager.as_str().is_empty());
            assert_eq!(PackageManager::from_name(manager.as_str()), Some(manager));
        }
    }

    #[test]
    fn the_level_and_the_reason_are_decided_in_one_place() {
        // Every arm says `Generic` only where SURE read the manifest, which is
        // the only case in which it can find how the project is built and run.
        let read = ManifestState::Read(Box::new(package(json!({}))));
        assert_eq!(grade(&read, true).0, SupportLevel::Generic);
        assert_eq!(grade(&read, false).0, SupportLevel::Generic);
        assert_eq!(
            grade(&ManifestState::Absent, true).0,
            SupportLevel::InspectOnly
        );
        assert_eq!(
            grade(&ManifestState::Absent, false).0,
            SupportLevel::InspectOnly
        );
        let unread = ManifestState::Unread(UnreadReason::NotParsed {
            detail: "expected value".to_owned(),
        });
        assert_eq!(grade(&unread, false).0, SupportLevel::InspectOnly);
        assert!(
            grade(&unread, false).1.contains("could not read it"),
            "an unread manifest must not be described as an absent one"
        );
        assert_ne!(
            grade(&ManifestState::Absent, false).1,
            grade(&unread, false).1,
            "the two are different findings and must read differently"
        );
    }
}

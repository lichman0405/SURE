//! What a Rust project declares about itself.
//!
//! See [`super`] for the rule this is all arranged around: **a manifest states a
//! request, not a fact.** Nothing here runs `cargo`, resolves a version, or
//! compiles anything.
//!
//! # The files it reads, and why those
//!
//! | File | What it answers |
//! |---|---|
//! | `Cargo.toml` | the whole of what the project declares, for the root package and for a workspace |
//! | `Cargo.lock` | that a resolver has run here |
//! | `rust-toolchain.toml`, `rust-toolchain` | which toolchain the project pins, and which components it asks for |
//! | `rustfmt.toml`, `.rustfmt.toml` | that the project configures its formatter |
//! | `clippy.toml`, `.clippy.toml` | that the project configures its linter |
//!
//! `Cargo.lock` is **not parsed**, for the reason `node.rs` gives about
//! lockfiles: its contents are a resolved graph SURE has no use for, and what is
//! wanted from it is one bit. The four `rustfmt`/`clippy` configuration files are
//! not parsed either — they are read for the question *is this tool configured
//! here*, which their existence answers.
//!
//! # A `[package]` table and a `[workspace]` table are siblings
//!
//! They are two tables of one document, and either can be there without the
//! other. A `Cargo.toml` with only `[workspace]` is a *virtual manifest* — how a
//! workspace that is not itself a crate is written — and it declares a great
//! deal: the members, their shared dependencies, their shared lint levels. It is
//! graded exactly as a manifest with a package is, and the distinction that
//! matters is drawn at the commands: `cargo run` needs a package to run.
//!
//! **The workspace tables are held on the document and not on the package**, and
//! that is why [`Manifest`] is a separate type from [`PackageSection`]. Hanging
//! them off the package would lose a virtual manifest's entire workspace, which
//! is the case such a manifest exists for.
//!
//! # Three things this module refuses to infer
//!
//! **A `build.rs` is a program, and it is reported and never run.** Cargo
//! compiles and executes it before the crate, which makes it the one file in a
//! Rust project that runs arbitrary code at build time — the same standing
//! `setup.py` has in [`super::python`]. It appears in
//! [`RustProject::conventional_targets`] as a target of kind
//! [`TargetKind::BuildScript`] and nothing here executes or reads it.
//!
//! **An `exclude` list is not subtracted from the member list.** Whether a
//! directory named by both `members` and `exclude` is a member is Cargo's rule
//! and SURE has not read it, so both facts are reported and neither is applied:
//! [`Workspaces::excluded_members`] is the overlap, as data. Reporting the
//! overlap is reversible by a reader who knows the rule; silently honouring it
//! would be SURE asserting a rule it cannot cite.
//!
//! **A `src/lib.rs` is not read and not inferred from.** It is *reported*, as a
//! conventional target, because Cargo discovers a target there without a table —
//! but the report says what is at the path and never what the file contains.
//! This is recorded as a gap in `docs/architecture/ECOSYSTEM_DISCOVERY.md`,
//! because it is the one place this module leans on a Cargo convention rather
//! than on a declaration.

use std::path::{Path, PathBuf};

use serde_json::Value;
use sure_domain::vocabulary::SupportLevel;

use super::pattern;
use super::read::{self, Budget, Probe, ReadFile, UnreadReason};
use super::{
    DiscoverOptions, Ecosystem, EcosystemReport, Findings, MemberManifest, Source, Unread,
};
use crate::scan::Scan;

/// The manifest every Rust project has, relative to the project root.
const MANIFEST: &str = "Cargo.toml";

/// The lockfile, whose existence says a resolver has run here.
const LOCKFILE: &str = "Cargo.lock";

/// The two names a pinned toolchain goes under.
///
/// `rust-toolchain.toml` is the current spelling and holds a `[toolchain]`
/// table; the bare `rust-toolchain` is the older one and usually holds nothing
/// but the channel. Both are read, and which one was found is carried in the
/// result, because the two say different amounts.
const TOOLCHAIN_FILES: &[&str] = &["rust-toolchain.toml", "rust-toolchain"];

/// The names a formatter or linter configuration goes under.
const FORMATTER_CONFIGS: &[&str] = &["rustfmt.toml", ".rustfmt.toml"];
const LINTER_CONFIGS: &[&str] = &["clippy.toml", ".clippy.toml"];

/// Whether a conventional target path is a file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// One file.
    File,
    /// A directory whose entries are each a target.
    Directory,
}

/// The paths Cargo finds a target at without a table naming one.
///
/// Reported as what is *at* the path, and never as what the file contains. The
/// last of them is the one worth pausing on: `build.rs` is compiled and run
/// before the crate, so it is the only thing on this list that is a program.
const CONVENTIONAL_TARGETS: &[(&str, TargetKind, Shape)] = &[
    ("src/lib.rs", TargetKind::Library, Shape::File),
    ("src/main.rs", TargetKind::Binary, Shape::File),
    ("src/bin", TargetKind::Binary, Shape::Directory),
    ("examples", TargetKind::Example, Shape::Directory),
    ("tests", TargetKind::Test, Shape::Directory),
    ("benches", TargetKind::Benchmark, Shape::Directory),
    ("build.rs", TargetKind::BuildScript, Shape::File),
];

/// Which of Cargo's dependency tables a dependency was declared in.
///
/// SURE's own vocabulary, mapped onto the manifest's keys by [`Self::table`]. A
/// dependency is not moved between kinds by this module and no kind is inferred
/// from a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyKind {
    /// `[dependencies]` — needed to build the crate.
    Runtime,
    /// `[dev-dependencies]` — needed to build this crate's tests, examples and
    /// benchmarks, and not needed by anything that depends on it.
    Development,
    /// `[build-dependencies]` — needed to run `build.rs`.
    Build,
}

impl DependencyKind {
    /// Every kind, in the order the tables are written in a manifest.
    pub const ALL: &'static [Self] = &[Self::Runtime, Self::Development, Self::Build];

    /// The stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Development => "development",
            Self::Build => "build",
        }
    }

    /// The manifest key this kind is written under.
    ///
    /// A constant, so that no dependency kind can reach a finding as text a
    /// project wrote.
    #[must_use]
    pub const fn table(self) -> &'static str {
        match self {
            Self::Runtime => "dependencies",
            Self::Development => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }

    /// What a person would call it.
    #[must_use]
    pub const fn plain_name(self) -> &'static str {
        match self {
            Self::Runtime => "a dependency of the crate itself",
            Self::Development => "a dependency of the tests and examples",
            Self::Build => "a dependency of the build script",
        }
    }
}

/// Where a dependency's version constraint is written, if it is written at all.
///
/// Four arms and not an `Option<String>`, because the three ways a dependency
/// can have no version in *this* file are three different facts. `foo = "1"`,
/// `foo = { workspace = true }` and `foo = { path = "../foo" }` all leave the
/// question "what version?" unanswered, and answering them the same way is how
/// a workspace member's dependency comes to look unconstrained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement {
    /// The manifest wrote this, verbatim, and SURE has not resolved it.
    Stated(String),
    /// The manifest wrote `workspace = true`, so the constraint is in the
    /// workspace's `[workspace.dependencies]` table.
    ///
    /// SURE reads that table and does **not** join it onto this entry: the two
    /// are separate files' worth of fact, and a joined value could not say which
    /// file a constraint came from.
    FromWorkspace,
    /// The manifest named the dependency and wrote no constraint — a `path` or
    /// `git` dependency, or one with nothing but a feature list.
    Unstated,
    /// There is a value and SURE cannot read it as a constraint.
    ///
    /// The dependency is still reported, with its name: "the project declared
    /// this and SURE cannot read how" and "the project declared no such
    /// dependency" are different facts, and dropping the entry would report the
    /// second.
    NotReadable(&'static str),
}

impl Requirement {
    /// The constraint, if the manifest stated one.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Stated(text) => Some(text),
            Self::FromWorkspace | Self::Unstated | Self::NotReadable(_) => None,
        }
    }

    /// The stable name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Stated(_) => "stated",
            Self::FromWorkspace => "from_workspace",
            Self::Unstated => "unstated",
            Self::NotReadable(_) => "not_readable",
        }
    }
}

/// One dependency the project declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The name, exactly as the manifest wrote it.
    pub name: String,
    /// Where the constraint is written, if it is written here.
    pub requirement: Requirement,
    /// Which table it was in.
    pub kind: DependencyKind,
    /// The `target` table it was under, verbatim.
    ///
    /// `[target.'cfg(unix)'.dependencies]` is a dependency that is only there on
    /// some platforms. The text is carried as the project wrote it and is never
    /// evaluated — SURE has not evaluated a `cfg` and is not going to start.
    pub target: Option<String>,
}

/// One feature a package declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    /// The feature's name, as written.
    pub name: String,
    /// What it turns on, verbatim — `"dep:serde"`, `"other/feature"`, another
    /// feature's name. SURE does not resolve any of them.
    pub enables: Vec<String>,
    /// Whether the feature is on unless something turns it off.
    pub default: bool,
}

/// What kind of target a table or a conventional path names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetKind {
    /// `[lib]`.
    Library,
    /// `[[bin]]`, or `src/main.rs`, or `src/bin`.
    Binary,
    /// `[[example]]`, or `examples`.
    Example,
    /// `[[test]]`, or `tests`.
    Test,
    /// `[[bench]]`, or `benches`.
    Benchmark,
    /// `build.rs` — a program Cargo runs before the crate.
    BuildScript,
}

impl TargetKind {
    /// Every kind.
    pub const ALL: &'static [Self] = &[
        Self::Library,
        Self::Binary,
        Self::Example,
        Self::Test,
        Self::Benchmark,
        Self::BuildScript,
    ];

    /// The stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::Binary => "binary",
            Self::Example => "example",
            Self::Test => "test",
            Self::Benchmark => "benchmark",
            Self::BuildScript => "build_script",
        }
    }

    /// What a person would call it.
    #[must_use]
    pub const fn plain_name(self) -> &'static str {
        match self {
            Self::Library => "a library",
            Self::Binary => "a program",
            Self::Example => "an example",
            Self::Test => "a test target",
            Self::Benchmark => "a benchmark",
            Self::BuildScript => "a build script, which Cargo runs before the crate",
        }
    }

    /// The manifest table this kind is written under, for the kinds that have
    /// one.
    #[must_use]
    pub const fn table(self) -> Option<&'static str> {
        match self {
            Self::Library => Some("lib"),
            Self::Binary => Some("bin"),
            Self::Example => Some("example"),
            Self::Test => Some("test"),
            Self::Benchmark => Some("bench"),
            Self::BuildScript => None,
        }
    }
}

/// A target declared in a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// What kind of target it is.
    pub kind: TargetKind,
    /// The name the manifest gave it, if it gave one.
    ///
    /// `None` is ordinary: a `[[bin]]` with only a `path` takes its name from
    /// the file. SURE does not derive that name.
    pub name: Option<String>,
    /// The manifest key the entry was written under, as a constant.
    pub declared_at: &'static str,
}

/// A target found at a path Cargo discovers one at without a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalTarget {
    /// The path, relative to the project root.
    pub path: PathBuf,
    /// What Cargo discovers there.
    pub kind: TargetKind,
}

/// What a manifest says about `build.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BuildScript {
    /// `build = "..."` names a path.
    At(String),
    /// `build = false` — the project says it has no build script, whatever is
    /// on disk.
    Disabled,
    /// The key is not there, so Cargo's own rule decides.
    #[default]
    Unstated,
}

/// One lint level a manifest sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintSetting {
    /// `rust` or `clippy`.
    pub tool: &'static str,
    /// The lint or group, as written.
    pub name: String,
    /// The level, as written.
    pub level: String,
}

/// The `[package]` table of a manifest.
///
/// Every field is optional because every field is optional in Cargo, and an
/// `Option` here means *the manifest did not write it* rather than *SURE could
/// not read it* — a distinction that holds because the whole table is parsed at
/// once and a shape failure leaves the manifest [`ManifestState::Unread`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageSection {
    /// The package's name.
    pub name: Option<String>,
    /// The package's version, verbatim.
    pub version: Option<String>,
    /// The edition, verbatim — `"2024"` is not read as a number.
    pub edition: Option<String>,
    /// The minimum supported Rust version, verbatim. A range, and never
    /// resolved.
    pub rust_version: Option<String>,
    /// `publish = false`, when the manifest says so.
    pub publishes: Option<bool>,
    /// What the manifest says about `build.rs`.
    pub build: BuildScript,
    /// Every dependency, across all three tables, sorted.
    pub dependencies: Vec<Dependency>,
    /// Every feature declared.
    pub features: Vec<Feature>,
    /// Every target a table names.
    pub targets: Vec<Target>,
    /// Every lint level the package sets.
    pub lints: Vec<LintSetting>,
}

impl PackageSection {
    /// The dependencies declared in one table.
    pub fn dependencies_of(&self, kind: DependencyKind) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(move |entry| entry.kind == kind)
    }

    /// Whether the package declares a target of this kind, by table.
    #[must_use]
    pub fn has_target(&self, kind: TargetKind) -> bool {
        self.targets.iter().any(|target| target.kind == kind)
    }

    /// The lint levels set for one tool.
    pub fn lints_of(&self, tool: &'static str) -> impl Iterator<Item = &LintSetting> {
        self.lints
            .iter()
            .filter(move |setting| setting.tool == tool)
    }
}

/// The `[workspace]` table as it was written, before anything is resolved.
///
/// Apart from [`Workspaces`], which is this plus what the patterns resolved to.
/// The two exist separately because the tables have to be readable **when there
/// is no `[package]` table at all** — which is what a virtual manifest is — and
/// resolution needs the walk, which parsing does not have.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceTables {
    /// Whether a `[workspace]` table is there at all.
    ///
    /// Kept apart from [`Self::patterns`] being empty: `[workspace]` on its own
    /// is a real thing to write — it opts a package out of an enclosing
    /// workspace — and reporting it as "no workspace" would lose the fact.
    pub declared: bool,
    /// The `members` patterns, verbatim and in order.
    pub patterns: Vec<String>,
    /// The `exclude` patterns, verbatim. **Not applied**; see
    /// [`Workspaces::excluded_members`].
    pub excluded: Vec<String>,
    /// The `default-members` patterns, verbatim.
    pub default_members: Vec<String>,
    /// The keys of `[workspace.dependencies]`, sorted.
    ///
    /// The names only. A member saying `foo = { workspace = true }` is merged
    /// with these by Cargo and by nothing here.
    pub inherited: Vec<String>,
    /// Every lint level `[workspace.lints]` sets.
    ///
    /// A virtual manifest's only place to set them, and the reason a workspace
    /// that configures clippy is one that runs it.
    pub lints: Vec<LintSetting>,
}

/// The `[workspace]` table, and what its patterns resolved to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Workspaces {
    /// The table as written.
    pub tables: WorkspaceTables,
    /// What said so.
    pub declared_by: Vec<Source>,
    /// The directories the patterns named, in resolution order.
    pub members: Vec<Member>,
    /// The members SURE resolved that `exclude` also names.
    ///
    /// Reported rather than subtracted. Whether this removes them is Cargo's
    /// rule and SURE has not read it, so both facts are here and neither is
    /// applied to [`Self::members`].
    pub excluded_members: Vec<PathBuf>,
    /// The patterns that named nothing, and why.
    pub unresolved: Vec<pattern::Unresolved>,
    /// Whether the member list was cut short.
    ///
    /// `true` means there were more members than
    /// [`DiscoverOptions::max_workspace_members`] and this list is a prefix of
    /// what the patterns named. A truncated member list is a workspace SURE has
    /// not finished looking at, and it says so rather than presenting a shorter
    /// workspace as the whole one.
    pub truncated: bool,
}

impl Workspaces {
    /// Whether the project declares a workspace.
    #[must_use]
    pub fn is_declared(&self) -> bool {
        self.tables.declared
    }

    /// Whether the workspace has a member SURE can name.
    #[must_use]
    pub fn names_any_member(&self) -> bool {
        !self.members.is_empty()
    }

    /// The members whose own manifest SURE read.
    pub fn readable_members(&self) -> impl Iterator<Item = &Member> {
        self.members
            .iter()
            .filter(|member| member.package.is_some())
    }

    /// The lint levels the workspace sets for one tool.
    pub fn lints_of(&self, tool: &'static str) -> impl Iterator<Item = &LintSetting> {
        self.tables
            .lints
            .iter()
            .filter(move |setting| setting.tool == tool)
    }
}

/// One directory a workspace pattern named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The directory, relative to the project root.
    pub path: PathBuf,
    /// What is at its `Cargo.toml`, without reading it.
    pub manifest: MemberManifest,
    /// The member's own manifest, read.
    ///
    /// Read because a workspace's frameworks and test tools are usually not in
    /// the root manifest: a root declaring nothing but `[workspace]` says
    /// nothing about the crate in `crates/parser`. Kept per member rather than
    /// merged into the root's, because the two are different manifests and a
    /// merged list could not say which declared what.
    pub package: Option<Box<PackageSection>>,
}

/// A `Cargo.toml`, read.
///
/// The document rather than the package, because the two tables it can hold are
/// siblings and either can be there without the other.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// The `[package]` table, if there is one. Absent in a virtual manifest.
    pub package: Option<PackageSection>,
    /// The `[workspace]` table as written.
    pub workspace_tables: WorkspaceTables,
}

/// Whether the root's `Cargo.toml` was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestState {
    /// There is a manifest and this is what it says.
    Read(Box<Manifest>),
    /// There is nothing at that name.
    Absent,
    /// There is a manifest and SURE did not get a value out of it.
    Unread(UnreadReason),
}

impl ManifestState {
    /// The manifest, if it was read.
    #[must_use]
    pub fn manifest(&self) -> Option<&Manifest> {
        match self {
            Self::Read(manifest) => Some(manifest),
            Self::Absent | Self::Unread(_) => None,
        }
    }

    /// The `[package]` table, if the manifest was read and has one.
    #[must_use]
    pub fn package(&self) -> Option<&PackageSection> {
        self.manifest()
            .and_then(|manifest| manifest.package.as_ref())
    }

    /// Whether anything at all was at that name.
    #[must_use]
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// What the project pins its toolchain to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    /// Which of the two names it was found at.
    pub file: &'static str,
    /// `channel`, verbatim. For the bare form this is the file's whole text.
    pub channel: Option<String>,
    /// `components`, verbatim — the list that makes `cargo clippy` a command
    /// the project asked for rather than one SURE assumed.
    pub components: Vec<String>,
    /// `targets`, verbatim.
    pub targets: Vec<String>,
    /// `profile`, verbatim.
    pub profile: Option<String>,
}

/// Whether a toolchain is pinned, and if not, whether that is known.
///
/// Three arms for the reason the whole module has three arms everywhere: a
/// `rust-toolchain.toml` that could not be read must not arrive as a project
/// that pins nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainState {
    /// There is a pin and this is what it says.
    Read(Box<Toolchain>),
    /// There is nothing at either name.
    Absent,
    /// There is a file at one of the names and SURE did not get a value out of
    /// it.
    Unread(UnreadReason),
}

impl ToolchainState {
    /// The pin, if there was one and it was read.
    #[must_use]
    pub fn toolchain(&self) -> Option<&Toolchain> {
        match self {
            Self::Read(toolchain) => Some(toolchain),
            Self::Absent | Self::Unread(_) => None,
        }
    }

    /// Whether anything at all was at either name.
    #[must_use]
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// Whether the project asked for a component by name.
    ///
    /// Matched case-insensitively against the component list, because the list
    /// is the project's text and `Clippy` is the same component as `clippy`.
    /// What is compared is the *constant* the caller passes, so no project text
    /// reaches a finding.
    #[must_use]
    pub fn asks_for(&self, component: &str) -> bool {
        self.toolchain().is_some_and(|toolchain| {
            toolchain
                .components
                .iter()
                .any(|asked| asked.eq_ignore_ascii_case(component))
        })
    }
}

/// What kind of thing a recognised crate is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolRole {
    /// A framework for answering HTTP requests.
    WebFramework,
    /// A runtime that runs asynchronous code.
    AsyncRuntime,
    /// A tool that runs tests rather than the test harness built into `cargo`.
    TestRunner,
    /// A tool that measures how fast code is rather than whether it is right.
    BenchmarkRunner,
}

impl ToolRole {
    /// Every role.
    pub const ALL: &'static [Self] = &[
        Self::WebFramework,
        Self::AsyncRuntime,
        Self::TestRunner,
        Self::BenchmarkRunner,
    ];

    /// The stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WebFramework => "web_framework",
            Self::AsyncRuntime => "async_runtime",
            Self::TestRunner => "test_runner",
            Self::BenchmarkRunner => "benchmark_runner",
        }
    }

    /// What a person would call it.
    #[must_use]
    pub const fn plain_name(self) -> &'static str {
        match self {
            Self::WebFramework => "a web framework",
            Self::AsyncRuntime => "an asynchronous runtime",
            Self::TestRunner => "a testing library",
            Self::BenchmarkRunner => "a benchmarking library",
        }
    }
}

/// The crates this module recognises, and what each one is.
///
/// Fixed, and the value stored in a [`Tooling`] is the table's own
/// `&'static str`, so a crate name a project wrote can never reach a sentence
/// SURE prints. A crate that is not in the table is not reported at all: "SURE
/// did not recognise this crate" is not a finding, and guessing from a name
/// would be.
///
/// A crate appears more than once only where it is genuinely two things —
/// `criterion` both runs and measures benchmarks — and both are reported,
/// because picking one would be SURE choosing which half of a tool to mention.
///
/// **What is deliberately not here**: `serde`, `rand`, `regex`, `log`, and every
/// other crate that is a plain library. They are dependencies, they are reported
/// as dependencies, and they do not change what SURE would run or how it reads
/// the project. A table grown to include them would be a list of crates SURE has
/// heard of, which is not a finding about anything.
const TOOLS: &[(&str, ToolRole)] = &[
    // Web frameworks.
    ("actix-web", ToolRole::WebFramework),
    ("axum", ToolRole::WebFramework),
    ("poem", ToolRole::WebFramework),
    ("rocket", ToolRole::WebFramework),
    ("salvo", ToolRole::WebFramework),
    ("tide", ToolRole::WebFramework),
    ("warp", ToolRole::WebFramework),
    // Asynchronous runtimes.
    ("async-std", ToolRole::AsyncRuntime),
    ("smol", ToolRole::AsyncRuntime),
    ("tokio", ToolRole::AsyncRuntime),
    // Testing libraries.
    ("insta", ToolRole::TestRunner),
    ("mockall", ToolRole::TestRunner),
    ("mockito", ToolRole::TestRunner),
    ("pretty_assertions", ToolRole::TestRunner),
    ("proptest", ToolRole::TestRunner),
    ("quickcheck", ToolRole::TestRunner),
    ("rstest", ToolRole::TestRunner),
    ("test-case", ToolRole::TestRunner),
    ("wiremock", ToolRole::TestRunner),
    // Benchmarking.
    ("criterion", ToolRole::BenchmarkRunner),
    ("divan", ToolRole::BenchmarkRunner),
    ("iai", ToolRole::BenchmarkRunner),
];

/// A recognised crate with one role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tooling {
    /// The table's own name, never the project's spelling.
    pub package: &'static str,
    /// What kind of thing it is.
    pub role: ToolRole,
    /// Which table it was declared in.
    pub kind: DependencyKind,
    /// The file it came from — the root manifest, or a member's.
    pub from: PathBuf,
}

/// One command SURE would run for a conventional role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommand {
    /// Which role.
    pub role: CommandRole,
    /// The command, or `None` when nothing the project declared would run it.
    pub command: Option<String>,
    /// Why the project is said to have this role, as sources.
    ///
    /// **Empty when there is no command.** A row that carried evidence for a
    /// command it does not have would say the project declared something it did
    /// not.
    pub because: Vec<Source>,
}

impl ConventionalCommand {
    /// Whether SURE would run something for this role.
    #[must_use]
    pub fn is_planned(&self) -> bool {
        self.command.is_some()
    }
}

/// The roles SURE looks for in any Rust project.
///
/// SURE's list, not the project's. A project with no way to check its own types
/// is a finding, and it can only be reported by looking for a name the project
/// did not choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandRole {
    /// Compile the crate.
    Build,
    /// Compile and run the tests.
    Test,
    /// Compile everything without producing a binary.
    Check,
    /// Look for probable mistakes.
    Lint,
    /// Rewrite the source to a style.
    Format,
    /// Build the documentation.
    Document,
    /// Run the program.
    Run,
    /// Run the benchmarks.
    Bench,
    /// Remove the build output.
    Clean,
}

impl CommandRole {
    /// Every role, in the order a person would reach for them.
    pub const ALL: &'static [Self] = &[
        Self::Build,
        Self::Test,
        Self::Check,
        Self::Lint,
        Self::Format,
        Self::Document,
        Self::Run,
        Self::Bench,
        Self::Clean,
    ];

    /// The stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Test => "test",
            Self::Check => "check",
            Self::Lint => "lint",
            Self::Format => "format",
            Self::Document => "document",
            Self::Run => "run",
            Self::Bench => "bench",
            Self::Clean => "clean",
        }
    }

    /// What a person would call it.
    #[must_use]
    pub const fn plain_name(self) -> &'static str {
        match self {
            Self::Build => "build the project",
            Self::Test => "run the tests",
            Self::Check => "check that it compiles",
            Self::Lint => "look for probable mistakes",
            Self::Format => "rewrite the source to a style",
            Self::Document => "build the documentation",
            Self::Run => "run the program",
            Self::Bench => "run the benchmarks",
            Self::Clean => "remove the build output",
        }
    }

    /// The crate or component that has to be there for this role to run.
    ///
    /// `cargo` for most of them, because `Cargo.toml` is `cargo`'s own file and
    /// a project with one has declared it. `clippy` and `rustfmt` are separate
    /// installs, and a role that names one of those is planned only where the
    /// project asked for it — see [`ToolEvidence`].
    #[must_use]
    pub const fn tool(self) -> &'static str {
        match self {
            Self::Lint => "clippy",
            Self::Format => "rustfmt",
            Self::Build
            | Self::Test
            | Self::Check
            | Self::Document
            | Self::Run
            | Self::Bench
            | Self::Clean => "cargo",
        }
    }
}

/// How a tool that is not `cargo` came to be one the project asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolEvidence {
    /// A `rust-toolchain.toml` names it as a component the project installs.
    ///
    /// The strongest of the three: the project pinned the toolchain *and* asked
    /// for this to be in it.
    ToolchainComponent,
    /// The project sets this tool's lint levels, in `[lints.clippy]` or
    /// `[workspace.lints.clippy]`.
    ///
    /// A project that decides what its linter should complain about has decided
    /// to run its linter. `[lints.rust]` is not this: it is the compiler's own
    /// lints, which `cargo check` reports.
    LintLevels,
    /// The project has a configuration file for it — `clippy.toml`,
    /// `rustfmt.toml`. A project that configures a tool has decided to run it.
    Configured,
}

impl ToolEvidence {
    /// The stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToolchainComponent => "toolchain_component",
            Self::LintLevels => "lint_levels",
            Self::Configured => "configured",
        }
    }

    /// The sentence a person reads.
    #[must_use]
    pub const fn plain_description(self) -> &'static str {
        match self {
            Self::ToolchainComponent => {
                "The project's toolchain file names this as a component it installs."
            }
            Self::LintLevels => "The project sets this tool's lint levels.",
            Self::Configured => "The project has a configuration file for this tool.",
        }
    }
}

/// Everything SURE found out about a Rust project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProject {
    /// What is at the project root's `Cargo.toml`.
    pub manifest: ManifestState,
    /// The workspace structure the root manifest declares, and what its patterns
    /// resolved to.
    pub workspaces: Workspaces,
    /// Whether a `Cargo.lock` is there.
    ///
    /// Presence and never contents, for the reason `node.rs` gives.
    pub lockfile: bool,
    /// The toolchain the project pins, or that it pins none.
    pub toolchain: ToolchainState,
    /// The formatter configuration file that is there, if one is.
    pub formatter_config: Option<&'static str>,
    /// The linter configuration file that is there, if one is.
    pub linter_config: Option<&'static str>,
    /// Recognised crates, from the root manifest and every member's.
    pub tooling: Vec<Tooling>,
    /// Paths Cargo discovers a target at without a table.
    pub conventional_targets: Vec<ConventionalTarget>,
}

impl RustProject {
    /// The root `[package]` table, if the manifest was read and has one.
    #[must_use]
    pub fn package(&self) -> Option<&PackageSection> {
        self.manifest.package()
    }

    /// Every recognised crate with one role.
    pub fn tooling_of_role(&self, role: ToolRole) -> impl Iterator<Item = &Tooling> {
        self.tooling.iter().filter(move |tool| tool.role == role)
    }

    /// Whether the project declared this crate anywhere.
    ///
    /// `package` is matched against the table's own names, so a caller asks
    /// about a crate SURE knows rather than about one a project wrote.
    #[must_use]
    pub fn declares(&self, package: &str) -> bool {
        self.tooling.iter().any(|tool| tool.package == package)
    }

    /// Whether the project has a target of this kind — by a table, by a
    /// conventional path, or in any member.
    #[must_use]
    pub fn has_target(&self, kind: TargetKind) -> bool {
        self.conventional_targets
            .iter()
            .any(|target| target.kind == kind)
            || self
                .package()
                .is_some_and(|package| package.has_target(kind))
            || self.workspaces.members.iter().any(|member| {
                member
                    .package
                    .as_deref()
                    .is_some_and(|package| package.has_target(kind))
            })
    }

    /// Every dependency of the root package, in table then name order.
    #[must_use]
    pub fn dependencies(&self) -> Vec<Dependency> {
        self.package()
            .map(|package| package.dependencies.clone())
            .unwrap_or_default()
    }

    /// The commands SURE would run for the conventional roles.
    #[must_use]
    pub fn conventional_commands(&self) -> Vec<ConventionalCommand> {
        conventional_commands(self)
    }
}

/// What a Rust project's support level is, and the sentence that says why.
///
/// Returned together, and as a constant, so that a level and its reason cannot
/// be assigned in two places and disagree.
fn grade(
    manifest: &ManifestState,
    declared_a_workspace: bool,
    found_a_lockfile: bool,
    found_a_toolchain: bool,
) -> (SupportLevel, &'static str) {
    match manifest {
        // A virtual manifest is graded here too, and deliberately: it is a
        // manifest SURE read, and what it declares — the members, their shared
        // dependencies and lint levels — is what the project declares.
        ManifestState::Read(_) => (
            SupportLevel::Generic,
            "SURE read this project's Cargo.toml, so it can find how the project is \
             built and run.",
        ),
        ManifestState::Unread(_) => (
            SupportLevel::InspectOnly,
            "There is a Cargo.toml here and SURE could not read it, so it can only look \
             at the project's files.",
        ),
        ManifestState::Absent if found_a_lockfile => (
            SupportLevel::InspectOnly,
            "This project has a Cargo.lock and no Cargo.toml SURE can read, so it is \
             plainly a Rust project and nothing here declares how it is built.",
        ),
        ManifestState::Absent if declared_a_workspace || found_a_toolchain => (
            SupportLevel::InspectOnly,
            "Something here says this is a Rust project and nothing SURE reads declares \
             anything, so it can only look at the project's files.",
        ),
        ManifestState::Absent => (
            SupportLevel::InspectOnly,
            "There is no Cargo.toml here, so SURE can see this project's files and \
             cannot read how it is built.",
        ),
    }
}

/// Find out what a Rust project declares, if this is one.
///
/// Returns `None` when nothing says this is one. See [`looks_like_one`] for the
/// complete list of what counts, which is short on purpose: `.rs` files are not
/// on it.
pub(super) fn look(
    root: &Path,
    scan: &Scan,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
) -> Option<EcosystemReport> {
    let tree = read::Tree::of(scan, options.scan.case);
    let mut found_by = Vec::new();

    // --- presence, before anything is read ----------------------------------
    //
    // Decided before any read and before any of the manifest budget is spent, so
    // that a directory which turns out not to be a Rust project has not already
    // put a file into `unread` — a finding about a project SURE then says
    // nothing about.
    let manifest_path = Path::new(MANIFEST);
    let lockfile_path = Path::new(LOCKFILE);
    let manifest_there = !matches!(tree.probe(manifest_path), Probe::Nothing);
    let lockfile_there = matches!(tree.probe(lockfile_path), Probe::File(_));
    let toolchain_at = TOOLCHAIN_FILES
        .iter()
        .copied()
        .find(|name| !matches!(tree.probe(Path::new(name)), Probe::Nothing));

    if !looks_like_one(manifest_there, lockfile_there, toolchain_at.is_some()) {
        return None;
    }

    if lockfile_there {
        found_by.push(lockfile_path.to_path_buf());
    }

    // --- the manifest -------------------------------------------------------
    let manifest = read_manifest(root, manifest_path, &tree, options, budget, unread);
    if !manifest.is_absent() {
        found_by.push(manifest_path.to_path_buf());
    }

    // --- the workspace ------------------------------------------------------
    let mut workspaces = Workspaces::default();
    if let Some(read) = manifest.manifest() {
        workspaces.tables = read.workspace_tables.clone();
        if workspaces.tables.declared {
            workspaces.declared_by.push(Source::new(
                MANIFEST,
                "has a workspace table naming this project's members",
            ));
        }
    }
    resolve_members(root, &tree, options, budget, unread, &mut workspaces);

    // --- the toolchain ------------------------------------------------------
    let toolchain = match toolchain_at {
        None => ToolchainState::Absent,
        Some(name) => {
            found_by.push(PathBuf::from(name));
            read_toolchain(root, name, &tree, options, budget, unread)
        }
    };

    // --- the formatter and linter configurations, by existence --------------
    //
    // Not read. These are text formats SURE does not need, and the question
    // asked of them is whether the project configured the tool, which their
    // existence answers.
    let formatter_config = FORMATTER_CONFIGS
        .iter()
        .copied()
        .find(|name| matches!(tree.probe(Path::new(name)), Probe::File(_)));
    let linter_config = LINTER_CONFIGS
        .iter()
        .copied()
        .find(|name| matches!(tree.probe(Path::new(name)), Probe::File(_)));
    for name in [formatter_config, linter_config].into_iter().flatten() {
        found_by.push(PathBuf::from(name));
    }

    // --- the targets Cargo finds without a table ----------------------------
    let conventional_targets = conventional_targets(&tree);

    // --- the crates, from every list of dependencies ------------------------
    let mut tooling = Vec::new();
    if let Some(package) = manifest.package() {
        collect_tooling(&package.dependencies, Path::new(MANIFEST), &mut tooling);
    }
    for member in workspaces.readable_members() {
        if let Some(package) = member.package.as_deref() {
            collect_tooling(
                &package.dependencies,
                &member.path.join(MANIFEST),
                &mut tooling,
            );
        }
    }
    tooling.sort_by(|a, b| {
        a.package
            .cmp(b.package)
            .then(a.role.cmp(&b.role))
            .then(a.from.cmp(&b.from))
    });
    tooling.dedup();

    let (level, reason) = grade(
        &manifest,
        workspaces.is_declared(),
        lockfile_there,
        toolchain_at.is_some(),
    );

    Some(EcosystemReport {
        ecosystem: Ecosystem::Rust,
        level,
        reason: reason.to_owned(),
        found_by,
        findings: Findings::Rust(Box::new(RustProject {
            manifest,
            workspaces,
            lockfile: lockfile_there,
            toolchain,
            formatter_config,
            linter_config,
            tooling,
            conventional_targets,
        })),
    })
}

/// Whether anything here says this is a Rust project.
///
/// Three markers, every one of them a project-level file: a manifest, a
/// lockfile, a pinned toolchain. **A `.rs` file is not on the list.** A
/// directory containing a `.rs` file is a directory containing a `.rs` file, and
/// calling that a Rust project on the strength of one stray file is the kind of
/// inference this module exists not to make.
fn looks_like_one(manifest_there: bool, lockfile_there: bool, toolchain_there: bool) -> bool {
    manifest_there || lockfile_there || toolchain_there
}

/// The targets Cargo discovers at conventional paths, without a table.
///
/// What is reported is what is *at* the path — a file, or a directory — and
/// never what any file contains. `build.rs` is the one that is a program.
fn conventional_targets(tree: &read::Tree<'_>) -> Vec<ConventionalTarget> {
    let mut found = Vec::new();
    for &(name, kind, shape) in CONVENTIONAL_TARGETS {
        let path = Path::new(name);
        let there = match shape {
            Shape::File => matches!(tree.probe(path), Probe::File(_)),
            Shape::Directory => tree.is_directory(path),
        };
        if there {
            found.push(ConventionalTarget {
                path: PathBuf::from(name),
                kind,
            });
        }
    }
    found
}

/// Read one manifest, pairing the reader with the converter.
///
/// Paired here rather than at each call site so that a shape failure cannot
/// reach the state without reaching [`Unread`]: everything that is not a table
/// is a document that is not a manifest, and reporting it as a project that
/// declares nothing is the false green this whole module is arranged against.
fn read_manifest(
    root: &Path,
    relative: &Path,
    tree: &read::Tree<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
) -> ManifestState {
    match read::read_toml(root, tree.probe(relative), options, budget) {
        ReadFile::Absent => ManifestState::Absent,
        ReadFile::Unread(reason) => record(reason, relative, unread),
        ReadFile::Parsed(value) => match Manifest::from_json(&value) {
            Ok(manifest) => ManifestState::Read(Box::new(manifest)),
            Err(reason) => record(reason, relative, unread),
        },
    }
}

/// Put a reason in the result and return the state that goes with it.
///
/// One function so that the path and the reason cannot get out of step: every
/// arm that reports a file as unread has to say *which* file, and a state built
/// without the push would be a file that went missing with nothing to say it
/// had.
fn record(reason: UnreadReason, relative: &Path, unread: &mut Vec<Unread>) -> ManifestState {
    unread.push(Unread {
        path: relative.to_path_buf(),
        reason: reason.clone(),
    });
    ManifestState::Unread(reason)
}

/// Resolve every member pattern to the directories it names, and read their
/// manifests.
fn resolve_members(
    root: &Path,
    tree: &read::Tree<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
    workspaces: &mut Workspaces,
) {
    let (mut ordered, unresolved) =
        pattern::resolve(tree, &workspaces.tables.patterns, options.scan.case);
    workspaces.unresolved = unresolved;

    // Truncation is recorded rather than applied quietly: a workspace whose
    // members are cut off is a workspace SURE has not finished looking at, and
    // `truncated` is what says so.
    if ordered.len() > options.max_workspace_members {
        ordered.truncate(options.max_workspace_members);
        workspaces.truncated = true;
    }

    // The overlap with `exclude`, worked out against the resolved list and
    // **not** applied to it. See `Workspaces::excluded_members`.
    let mut excluded_members = Vec::new();
    for pattern in &workspaces.tables.excluded {
        if let Ok(paths) = pattern::expand(tree, pattern, options.scan.case) {
            for path in paths {
                if ordered.contains(&path) {
                    excluded_members.push(path);
                }
            }
        }
    }
    excluded_members.sort();
    excluded_members.dedup();
    workspaces.excluded_members = excluded_members;

    for path in ordered {
        let manifest_path = path.join(MANIFEST);
        let summary = match tree.probe(&manifest_path) {
            Probe::Nothing => MemberManifest::Absent,
            Probe::Other(kind) => MemberManifest::NotReadable(kind),
            Probe::File(_) => MemberManifest::Present,
        };
        // Read even when the summary already says what is there, because the
        // summary answers a different question: it says whether a manifest
        // exists, and this says what it declares.
        let package = match read_manifest(root, &manifest_path, tree, options, budget, unread) {
            ManifestState::Read(manifest) => manifest.package,
            // An absent or unread member manifest is already recorded:
            // `summary` says which of the two it is, and an unread one is in
            // `unread` with its path. Nothing is dropped by not repeating it.
            ManifestState::Absent | ManifestState::Unread(_) => None,
        };
        workspaces.members.push(Member {
            path,
            manifest: summary,
            package: package.map(Box::new),
        });
    }
}

/// Read the toolchain pin, from whichever of the two names it is at.
fn read_toolchain(
    root: &Path,
    name: &'static str,
    tree: &read::Tree<'_>,
    options: &DiscoverOptions,
    budget: &mut Budget,
    unread: &mut Vec<Unread>,
) -> ToolchainState {
    let unreadable = |reason: UnreadReason, unread: &mut Vec<Unread>| {
        unread.push(Unread {
            path: PathBuf::from(name),
            reason: reason.clone(),
        });
        ToolchainState::Unread(reason)
    };

    match read::read_text_file(root, tree.probe(Path::new(name)), options, budget) {
        ReadFile::Absent => ToolchainState::Absent,
        ReadFile::Unread(reason) => unreadable(reason, unread),
        ReadFile::Parsed(value) => {
            let Some(text) = value.as_str() else {
                return unreadable(UnreadReason::WrongShape { found: "text" }, unread);
            };
            match toolchain_from_text(name, text) {
                Ok(toolchain) => ToolchainState::Read(Box::new(toolchain)),
                Err(reason) => unreadable(reason, unread),
            }
        }
    }
}

/// Turn a toolchain file's text into a [`Toolchain`].
///
/// Two forms, and which one this is depends on **both** the text and the name.
///
/// A file whose text parses into a `[toolchain]` table is one, under either
/// name — a `rust-toolchain` holding a table is read as a table, because that is
/// what it says it is.
///
/// The bare form is a channel on a line by itself, and it is accepted only under
/// the name `rust-toolchain`. That name predates the TOML file and is not
/// defined as TOML at all, so text there need not parse. `rust-toolchain.toml`
/// *is* defined as TOML, and text under that name which does not parse is a file
/// SURE could not read — reporting its whole text as a version would be a
/// version number SURE made up, which is exactly the kind of finding this
/// product exists not to produce.
///
/// The bare form is one line with no spaces in it. A file with more in it is not
/// the file rustup documents, and taking its first line would silently drop the
/// rest of whatever somebody wrote.
fn toolchain_from_text(name: &'static str, text: &str) -> Result<Toolchain, UnreadReason> {
    let mut toolchain = Toolchain {
        file: name,
        channel: None,
        components: Vec::new(),
        targets: Vec::new(),
        profile: None,
    };

    match toml::from_str::<toml::Value>(text) {
        Ok(value) => {
            let table = value.get("toolchain").and_then(|table| table.as_table());
            if let Some(table) = table {
                toolchain.channel = text_of(table.get("channel"));
                toolchain.components = strings_of(table.get("components"));
                toolchain.targets = strings_of(table.get("targets"));
                toolchain.profile = text_of(table.get("profile"));
                return Ok(toolchain);
            }
            if name.ends_with(".toml") {
                return Err(UnreadReason::WrongShape {
                    found: "a TOML file with no toolchain table in it",
                });
            }
        }
        Err(error) => {
            if name.ends_with(".toml") {
                return Err(UnreadReason::NotParsed {
                    detail: error.to_string(),
                });
            }
        }
    }

    let channel = text.trim();
    if channel.is_empty() || channel.lines().count() != 1 || channel.split_whitespace().count() != 1
    {
        return Err(UnreadReason::WrongShape {
            found: "a toolchain file that is not a channel",
        });
    }
    toolchain.channel = Some(channel.to_owned());
    Ok(toolchain)
}

/// The strings in a TOML array, in order. Anything that is not a string is
/// dropped rather than turned into one.
fn strings_of(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// A TOML string value, trimmed, or nothing.
fn text_of(value: Option<&toml::Value>) -> Option<String> {
    value
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Add every recognised crate in a dependency list.
///
/// The name that goes into a [`Tooling`] is the **table's**, found by comparing
/// the project's spelling against each row, so a crate a project wrote can never
/// reach a finding. A dependency that is not in the table is not reported.
fn collect_tooling(dependencies: &[Dependency], from: &Path, into: &mut Vec<Tooling>) {
    for dependency in dependencies {
        for &(package, role) in TOOLS {
            if dependency.name == package {
                into.push(Tooling {
                    package,
                    role,
                    kind: dependency.kind,
                    from: from.to_path_buf(),
                });
            }
        }
    }
}

/// The command SURE would run for each conventional role.
///
/// **A row for every role whether or not there is a command**, because "there
/// is no way to check this project's types" is a finding and a missing line is
/// not. A row with no command carries no reasons.
fn conventional_commands(project: &RustProject) -> Vec<ConventionalCommand> {
    let declared = ToolDeclarations::of(project);
    CommandRole::ALL
        .iter()
        .copied()
        .map(|role| {
            let mut because = declared.evidence_for(role.tool());
            let command = command_for(project, role, &declared);
            if command.is_none() {
                // A reason for a command that is not there would say the project
                // declared something it did not.
                because.clear();
            }
            ConventionalCommand {
                role,
                command,
                because,
            }
        })
        .collect()
}

/// Which of the tools that are not `cargo` the project asked for, and how.
///
/// A tool is here only if something the project *wrote* names it: a component in
/// the toolchain file, lint levels it set for the tool, or a configuration file
/// of its own. Nothing is inferred from the manifest being a manifest — that is
/// `cargo`, and `cargo` is handled by [`CommandRole::tool`] returning it.
struct ToolDeclarations {
    evidence: Vec<(&'static str, ToolEvidence, Source)>,
}

impl ToolDeclarations {
    fn of(project: &RustProject) -> Self {
        let mut evidence = Vec::new();

        if let Some(toolchain) = project.toolchain.toolchain() {
            for &tool in &["clippy", "rustfmt"] {
                if project.toolchain.asks_for(tool) {
                    evidence.push((
                        tool,
                        ToolEvidence::ToolchainComponent,
                        Source::new(toolchain.file, "names this as a component it installs"),
                    ));
                }
            }
        }
        if let Some(name) = project.linter_config {
            evidence.push((
                "clippy",
                ToolEvidence::Configured,
                Source::new(name, "is a configuration file for this tool"),
            ));
        }
        if let Some(name) = project.formatter_config {
            evidence.push((
                "rustfmt",
                ToolEvidence::Configured,
                Source::new(name, "is a configuration file for this tool"),
            ));
        }
        // `[lints.clippy]` on the package and `[workspace.lints.clippy]` on the
        // workspace are both a project stating its own lint levels, which says
        // it runs the linter. `[lints.rust]` is not evidence for clippy: it is
        // the compiler's own lints, which `cargo check` reports.
        if project
            .package()
            .is_some_and(|package| package.lints_of("clippy").next().is_some())
        {
            evidence.push((
                "clippy",
                ToolEvidence::LintLevels,
                Source::new(MANIFEST, "sets clippy's lint levels"),
            ));
        }
        if project.workspaces.lints_of("clippy").next().is_some() {
            evidence.push((
                "clippy",
                ToolEvidence::LintLevels,
                Source::new(MANIFEST, "sets clippy's lint levels for the workspace"),
            ));
        }

        // Strongest first, and deduplicated by tool and evidence so that one
        // tool named by two files does not read as two different claims.
        evidence.sort_by(|a, b| {
            a.0.cmp(b.0)
                .then(a.1.cmp(&b.1))
                .then(a.2.path.cmp(&b.2.path))
        });
        evidence.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        Self { evidence }
    }

    /// Why the project is said to have this tool.
    fn evidence_for(&self, tool: &str) -> Vec<Source> {
        self.evidence
            .iter()
            .filter(|(named, _, _)| *named == tool)
            .map(|(_, _, source)| source.clone())
            .collect()
    }

    /// Whether something the project wrote names this tool.
    fn declares(&self, tool: &str) -> bool {
        self.evidence.iter().any(|(named, _, _)| *named == tool)
    }
}

/// The command for one role, or `None` when nothing the project declared runs
/// it.
///
/// **Returned and never executed.** Choosing a command is a plan; running it is
/// a later step with its own authorisation.
fn command_for(
    project: &RustProject,
    role: CommandRole,
    declared: &ToolDeclarations,
) -> Option<String> {
    // A role is planned only where SURE read a manifest, because without one
    // there is nothing that says a `cargo` command would act on this project at
    // all. A virtual manifest passes this: a workspace SURE read is a workspace
    // `cargo build` acts on, building every member.
    project.manifest.manifest()?;

    match role {
        CommandRole::Lint if declared.declares("clippy") => {
            // `--all-targets` rather than a bare `cargo clippy`, and it is a
            // decision rather than a default: clippy without it does not lint
            // the test and example code, so the plan would report on a smaller
            // program than the one the project has.
            Some("cargo clippy --all-targets".to_owned())
        }
        CommandRole::Format if declared.declares("rustfmt") => Some("cargo fmt".to_owned()),
        CommandRole::Build => Some("cargo build".to_owned()),
        CommandRole::Test => Some("cargo test".to_owned()),
        CommandRole::Check => Some("cargo check --all-targets".to_owned()),
        CommandRole::Document => Some("cargo doc".to_owned()),
        CommandRole::Clean => Some("cargo clean".to_owned()),
        // A program to run. `cargo run` needs a package, and a virtual manifest
        // has none, so this asks for both a package and a binary target: either
        // one missing means there is nothing to run.
        CommandRole::Run
            if project.package().is_some() && project.has_target(TargetKind::Binary) =>
        {
            Some("cargo run".to_owned())
        }
        // A benchmark to run. `cargo bench` with no bench target is a command
        // that succeeds and does nothing, which is the shape of answer this
        // product exists not to give.
        CommandRole::Bench
            if project.has_target(TargetKind::Benchmark)
                || project
                    .tooling_of_role(ToolRole::BenchmarkRunner)
                    .next()
                    .is_some() =>
        {
            Some("cargo bench".to_owned())
        }
        CommandRole::Lint | CommandRole::Format | CommandRole::Run | CommandRole::Bench => None,
    }
}

impl Manifest {
    /// Read a `Cargo.toml`'s JSON-shaped value into a [`Manifest`].
    ///
    /// Written out rather than derived, for the reason `PyProject::from_json`
    /// gives: a `#[derive(Deserialize)]` would answer a missing key and a
    /// wrongly-typed key the same way, and this module's whole rule is that they
    /// are different facts.
    ///
    /// # Errors
    ///
    /// [`UnreadReason::WrongShape`] when the document is not a table — the one
    /// thing that makes it not a manifest at all.
    pub(super) fn from_json(value: &Value) -> Result<Self, UnreadReason> {
        let Some(root) = value.as_object() else {
            return Err(UnreadReason::WrongShape { found: "a table" });
        };

        let mut manifest = Self::default();

        // The workspace table first, and whether or not there is a package: a
        // `[workspace]` with no `[package]` is a virtual manifest, and reading
        // this inside the package branch would lose its entire workspace.
        if let Some(table) = root.get("workspace").and_then(Value::as_object) {
            manifest.workspace_tables = WorkspaceTables {
                declared: true,
                patterns: strings(table.get("members")),
                excluded: strings(table.get("exclude")),
                default_members: strings(table.get("default-members")),
                inherited: sorted_keys(table.get("dependencies")),
                lints: lints_in(table.get("lints")),
            };
        }

        let Some(table) = root.get("package").and_then(Value::as_object) else {
            return Ok(manifest);
        };

        let mut package = PackageSection {
            name: text(table.get("name")),
            version: text(table.get("version")),
            edition: text(table.get("edition")),
            rust_version: text(table.get("rust-version")),
            publishes: table.get("publish").and_then(Value::as_bool),
            build: build_script_of(table.get("build")),
            ..PackageSection::default()
        };

        // The three dependency tables, then the same three under each `target`.
        for &kind in DependencyKind::ALL {
            dependencies_in(
                root.get(kind.table()),
                kind,
                None,
                &mut package.dependencies,
            );
        }
        if let Some(targets) = root.get("target").and_then(Value::as_object) {
            for (target, table) in targets {
                let Some(table) = table.as_object() else {
                    continue;
                };
                for &kind in DependencyKind::ALL {
                    dependencies_in(
                        table.get(kind.table()),
                        kind,
                        Some(target.as_str()),
                        &mut package.dependencies,
                    );
                }
            }
        }
        package.dependencies.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.kind.cmp(&b.kind))
                .then(a.target.cmp(&b.target))
        });

        // Features. `default` is the one Cargo turns on by itself, and it is
        // reported by that name rather than by position.
        if let Some(features) = root.get("features").and_then(Value::as_object) {
            for (name, enables) in features {
                package.features.push(Feature {
                    name: name.clone(),
                    enables: strings(Some(enables)),
                    default: name == "default",
                });
            }
            package.features.sort_by(|a, b| a.name.cmp(&b.name));
        }

        // Targets named by a table. `[lib]` is one table; the other four are
        // arrays of tables, and a single table is accepted for them too rather
        // than being reported as nothing.
        for &kind in TargetKind::ALL {
            let Some(key) = kind.table() else {
                continue;
            };
            let Some(value) = root.get(key) else {
                continue;
            };
            match value {
                Value::Array(entries) => {
                    for entry in entries {
                        package.targets.push(Target {
                            kind,
                            name: entry.as_object().and_then(|table| text(table.get("name"))),
                            declared_at: key,
                        });
                    }
                }
                Value::Object(table) => package.targets.push(Target {
                    kind,
                    name: text(table.get("name")),
                    declared_at: key,
                }),
                _ => continue,
            }
        }

        package.lints = lints_in(root.get("lints"));
        manifest.package = Some(package);
        Ok(manifest)
    }
}

/// What a `build` key says.
///
/// `build = false` is the project saying it has no build script, and that is not
/// the same as the key being absent: with the key absent, whatever is at
/// `build.rs` is what Cargo uses.
fn build_script_of(value: Option<&Value>) -> BuildScript {
    match value {
        None => BuildScript::Unstated,
        Some(Value::Bool(false)) => BuildScript::Disabled,
        Some(value) => match text(Some(value)) {
            Some(path) => BuildScript::At(path),
            None => BuildScript::Disabled,
        },
    }
}

/// Read a `[lints]` table.
///
/// `clippy` and `rust` are the two tables Cargo defines. A third would be a tool
/// SURE does not know and is not reported, because naming it would be repeating
/// a project's text in SURE's own vocabulary.
fn lints_in(value: Option<&Value>) -> Vec<LintSetting> {
    let mut lints = Vec::new();
    let Some(table) = value.and_then(Value::as_object) else {
        return lints;
    };
    for tool in ["clippy", "rust"] {
        let Some(levels) = table.get(tool).and_then(Value::as_object) else {
            continue;
        };
        for (name, level) in levels {
            let Some(level) = text(Some(level)) else {
                continue;
            };
            lints.push(LintSetting {
                tool,
                name: name.clone(),
                level,
            });
        }
    }
    lints.sort_by(|a, b| a.tool.cmp(b.tool).then(a.name.cmp(&b.name)));
    lints
}

/// Read one dependency table into the list.
///
/// Every entry is reported. An entry whose value is not a string and not a table
/// becomes [`Requirement::NotReadable`] rather than being dropped, because
/// "the project declared this and SURE cannot read how" and "the project
/// declared no such dependency" are different facts.
fn dependencies_in(
    value: Option<&Value>,
    kind: DependencyKind,
    target: Option<&str>,
    into: &mut Vec<Dependency>,
) {
    let Some(table) = value.and_then(Value::as_object) else {
        return;
    };
    for (name, spec) in table {
        into.push(Dependency {
            name: name.clone(),
            requirement: requirement_of(spec),
            kind,
            target: target.map(str::to_owned),
        });
    }
}

/// Where a dependency's constraint is written.
fn requirement_of(spec: &Value) -> Requirement {
    if let Some(text) = spec.as_str() {
        return Requirement::Stated(text.to_owned());
    }
    if let Some(table) = spec.as_object() {
        // `workspace = true` first: a spec may carry both it and a feature list,
        // and the version is not in this file either way.
        if table.get("workspace").and_then(Value::as_bool) == Some(true) {
            return Requirement::FromWorkspace;
        }
        if let Some(version) = text(table.get("version")) {
            return Requirement::Stated(version);
        }
        // A `path` or `git` dependency with no version. There is nothing to
        // resolve and nothing SURE could say about which revision it is.
        return Requirement::Unstated;
    }
    Requirement::NotReadable(shape_of(spec))
}

/// A constant phrase naming the shape of a value that is not a constraint.
fn shape_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "nothing at all",
        Value::Bool(_) => "true or false",
        Value::Number(_) => "a number",
        Value::String(_) => "text",
        Value::Array(_) => "a list",
        Value::Object(_) => "a table",
    }
}

/// The keys of a JSON object, sorted. Used for `[workspace.dependencies]`, whose
/// values are not read here.
fn sorted_keys(value: Option<&Value>) -> Vec<String> {
    let Some(table) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = table.keys().cloned().collect();
    keys.sort();
    keys
}

/// A string value, trimmed, or nothing.
fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// The strings in a JSON array, in order. Anything that is not a string is
/// dropped rather than turned into one: a `members` list with a number in it is
/// a list SURE read the readable part of, and turning the number into text would
/// invent a directory name.
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A manifest read the way production reads one: TOML text, through the
    /// workspace's own converter, into the JSON-shaped value `from_json` takes.
    ///
    /// Going through `read::to_json` rather than writing `json!` literals is
    /// deliberate — the tests are then exercising the same conversion the walk
    /// does, so a `to_json` that turned a TOML table into something else would
    /// fail here rather than passing in a shape production never produces.
    fn manifest_of(text: &str) -> Manifest {
        let parsed: toml::Value = toml::from_str(text).expect("the fixture is valid TOML");
        Manifest::from_json(&read::to_json(&parsed)).expect("a Cargo.toml is a table")
    }

    /// A project whose root manifest is `text`, with everything that needs a walk
    /// left empty.
    fn project_of(text: &str) -> RustProject {
        let manifest = manifest_of(text);
        let mut tooling = Vec::new();
        if let Some(package) = manifest.package.as_ref() {
            collect_tooling(&package.dependencies, Path::new(MANIFEST), &mut tooling);
        }
        RustProject {
            manifest: ManifestState::Read(Box::new(manifest)),
            workspaces: Workspaces::default(),
            lockfile: false,
            toolchain: ToolchainState::Absent,
            formatter_config: None,
            linter_config: None,
            tooling,
            conventional_targets: Vec::new(),
        }
    }

    /// The command planned for one role.
    fn command(project: &RustProject, role: CommandRole) -> Option<String> {
        project
            .conventional_commands()
            .into_iter()
            .find(|row| row.role == role)
            .expect("there is a row for every role")
            .command
    }

    /// The names of the crates recognised in one manifest.
    fn declared_crates(text: &str) -> Vec<&'static str> {
        project_of(text)
            .tooling
            .into_iter()
            .map(|tool| tool.package)
            .collect()
    }

    #[test]
    fn a_document_that_is_not_a_table_is_not_read_as_an_empty_manifest() {
        // The false green this module exists against: `[]` is not a Cargo.toml,
        // and reading it as one that declares nothing would report a project
        // whose manifest is empty rather than one whose manifest is wrong.
        for value in [
            serde_json::json!([]),
            serde_json::json!("package"),
            serde_json::json!(7),
            serde_json::json!(null),
        ] {
            let read = Manifest::from_json(&value);
            assert!(
                matches!(read, Err(UnreadReason::WrongShape { .. })),
                "{value} was read as a manifest"
            );
        }
        // An empty table *is* a manifest. It declares nothing, which is a fact
        // about the project rather than about the document.
        let empty = Manifest::from_json(&serde_json::json!({})).expect("a table");
        assert!(empty.package.is_none());
        assert!(!empty.workspace_tables.declared);
    }

    #[test]
    fn a_workspace_table_is_read_when_there_is_no_package_table() {
        // A virtual manifest. Hanging the workspace off the package would lose
        // the whole of it, and this is the shape that exists to be declared.
        let manifest = manifest_of(
            r#"
            [workspace]
            members = ["crates/parser", "crates/serializer"]
            exclude = ["crates/experiments"]
            default-members = ["crates/parser"]
            resolver = "2"

            [workspace.dependencies]
            serde = "1"

            [workspace.lints.clippy]
            unwrap_used = "warn"
            "#,
        );
        assert!(
            manifest.package.is_none(),
            "a virtual manifest has no package table"
        );
        let tables = &manifest.workspace_tables;
        assert!(tables.declared);
        assert_eq!(tables.patterns, vec!["crates/parser", "crates/serializer"]);
        assert_eq!(tables.excluded, vec!["crates/experiments"]);
        assert_eq!(tables.default_members, vec!["crates/parser"]);
        assert_eq!(tables.inherited, vec!["serde"]);
        assert_eq!(tables.lints.len(), 1);
        assert_eq!(tables.lints[0].tool, "clippy");
        assert_eq!(tables.lints[0].name, "unwrap_used");
        assert_eq!(tables.lints[0].level, "warn");
    }

    #[test]
    fn an_explicit_workspace_table_with_no_members_is_not_no_workspace_table() {
        // `[workspace]` on its own is how a package opts out of an enclosing
        // workspace. Reporting it as "no workspace" would lose that, and
        // `resolver = "2"` belongs to the same table.
        let manifest = manifest_of(
            r#"
            [package]
            name = "standalone"

            [workspace]
            "#,
        );
        assert!(manifest.workspace_tables.declared);
        assert!(manifest.workspace_tables.patterns.is_empty());
    }

    #[test]
    fn the_three_ways_a_dependency_has_no_version_here_are_three_facts() {
        let package = project_of(
            r#"
            [package]
            name = "x"

            [dependencies]
            stated = "1.2"
            inherited = { workspace = true, features = ["derive"] }
            local = { path = "../local" }
            "#,
        );
        let manifest = package.package().expect("a package table");
        let requirement = |name: &str| {
            manifest
                .dependencies
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("no dependency named {name}"))
                .requirement
                .clone()
        };
        assert_eq!(requirement("stated"), Requirement::Stated("1.2".to_owned()));
        // `workspace = true` is checked before the version, and here it carries
        // a feature list too — which must not be read as a replacement for a
        // constraint that is not in this file.
        assert_eq!(requirement("inherited"), Requirement::FromWorkspace);
        assert_eq!(requirement("local"), Requirement::Unstated);
        assert_eq!(requirement("stated").text(), Some("1.2"));
        assert_eq!(requirement("inherited").text(), None);
        assert_eq!(requirement("local").text(), None);
    }

    #[test]
    fn a_dependency_sure_cannot_read_is_not_a_dependency_that_is_not_there() {
        let project = project_of(
            r#"
            [package]
            name = "x"

            [dependencies]
            readable = "1"
            odd = 7
            "#,
        );
        let manifest = project.package().expect("a package table");
        assert_eq!(
            manifest.dependencies.len(),
            2,
            "an entry SURE cannot read was dropped, which reports it as one the \
             project never declared"
        );
        let odd = manifest
            .dependencies
            .iter()
            .find(|entry| entry.name == "odd")
            .expect("the unreadable dependency is still named");
        assert_eq!(odd.requirement, Requirement::NotReadable("a number"));
        assert_eq!(odd.requirement.as_str(), "not_readable");
    }

    #[test]
    fn every_dependency_table_is_read_including_the_ones_under_a_target() {
        let project = project_of(
            r#"
            [package]
            name = "x"

            [dependencies]
            plain = "1"

            [dev-dependencies]
            testonly = "1"

            [build-dependencies]
            compiler = "1"

            [target.'cfg(unix)'.dependencies]
            unixonly = "1"
            "#,
        );
        let manifest = project.package().expect("a package table");
        let find = |name: &str| {
            manifest
                .dependencies
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("no dependency named {name}"))
        };
        assert_eq!(find("plain").kind, DependencyKind::Runtime);
        assert_eq!(find("testonly").kind, DependencyKind::Development);
        assert_eq!(find("compiler").kind, DependencyKind::Build);
        assert_eq!(find("plain").target, None);
        // The target text is carried verbatim and is never evaluated: SURE has
        // not evaluated a `cfg` and does not know whether this is in effect.
        assert_eq!(find("unixonly").target.as_deref(), Some("cfg(unix)"));
        assert_eq!(find("unixonly").kind, DependencyKind::Runtime);
        assert_eq!(
            manifest
                .dependencies_of(DependencyKind::Development)
                .count(),
            1
        );
    }

    #[test]
    fn a_target_table_is_read_whether_it_is_one_entry_or_an_array() {
        // `[lib]` is one table; `[[bin]]` is an array. A single `[bin]` is a
        // mistake a project can write, and it is read rather than reported as
        // nothing.
        let manifest = project_of(
            r#"
            [package]
            name = "x"

            [lib]
            name = "the_library"

            [[bin]]
            name = "one"

            [[bin]]
            path = "src/other.rs"

            [[bench]]
            name = "throughput"
            "#,
        );
        let package = manifest.package().expect("a package table");
        let binaries: Vec<&Target> = package
            .targets
            .iter()
            .filter(|target| target.kind == TargetKind::Binary)
            .collect();
        assert_eq!(binaries.len(), 2);
        assert_eq!(binaries[0].name.as_deref(), Some("one"));
        // A `[[bin]]` with only a path has no name in the manifest, and SURE
        // does not derive one from the file name.
        assert_eq!(binaries[1].name, None);
        assert_eq!(binaries[0].declared_at, "bin");
        assert!(package.has_target(TargetKind::Library));
        assert!(package.has_target(TargetKind::Benchmark));
        assert!(!package.has_target(TargetKind::Example));
        assert_eq!(TargetKind::Library.table(), Some("lib"));
        assert_eq!(TargetKind::BuildScript.table(), None);
    }

    #[test]
    fn build_false_is_not_the_same_as_the_key_being_absent() {
        // With `build = false` the project has said it has no build script. With
        // the key absent, whatever is at `build.rs` is what Cargo uses. Turning
        // the first into the second is how a project that disabled its build
        // script gets reported as one that has whatever is on disk.
        let disabled = project_of(
            r#"
            [package]
            name = "x"
            build = false
            "#,
        );
        assert_eq!(
            disabled.package().expect("a package").build,
            BuildScript::Disabled
        );
        let unstated = project_of(
            r#"
            [package]
            name = "x"
            "#,
        );
        assert_eq!(
            unstated.package().expect("a package").build,
            BuildScript::Unstated
        );
        let named = project_of(
            r#"
            [package]
            name = "x"
            build = "build/main.rs"
            "#,
        );
        assert_eq!(
            named.package().expect("a package").build,
            BuildScript::At("build/main.rs".to_owned())
        );
    }

    #[test]
    fn the_toolchain_pin_is_read_in_both_of_its_forms() {
        let table = toolchain_from_text(
            "rust-toolchain.toml",
            r#"
            [toolchain]
            channel = "1.83.0"
            components = ["clippy", "rustfmt", "rust-src"]
            targets = ["wasm32-unknown-unknown"]
            profile = "minimal"
            "#,
        )
        .expect("a toolchain table");
        assert_eq!(table.channel.as_deref(), Some("1.83.0"));
        assert_eq!(table.components, vec!["clippy", "rustfmt", "rust-src"]);
        assert_eq!(table.targets, vec!["wasm32-unknown-unknown"]);
        assert_eq!(table.profile.as_deref(), Some("minimal"));

        // The older form is nothing but the channel, and it is accepted under
        // the name that predates the TOML file.
        let bare =
            toolchain_from_text("rust-toolchain", "nightly-2025-01-15\n").expect("a bare channel");
        assert_eq!(bare.channel.as_deref(), Some("nightly-2025-01-15"));
        assert!(bare.components.is_empty());
        assert_eq!(bare.file, "rust-toolchain");

        // Same content, the other file name: still the table, because the text
        // parses into one.
        let in_the_old_name =
            toolchain_from_text("rust-toolchain", "[toolchain]\nchannel = \"stable\"\n")
                .expect("a toolchain table");
        assert_eq!(in_the_old_name.channel.as_deref(), Some("stable"));

        assert!(
            matches!(
                toolchain_from_text("rust-toolchain", "   \n"),
                Err(UnreadReason::WrongShape { .. })
            ),
            "an empty toolchain file is not a project pinning the empty string"
        );
    }

    #[test]
    fn a_toolchain_file_that_is_not_what_its_name_says_is_not_read_as_a_channel() {
        // `rust-toolchain.toml` is defined as TOML. A file under that name whose
        // text is not a `[toolchain]` table is a file SURE could not read, and
        // reporting its text as a version would be a version SURE invented —
        // which is what an earlier revision of this function did.
        for text in [
            "stable\n",
            "[toolchain\nchannel = \n",
            // Valid TOML with no `toolchain` table. `channel="stable"` is here
            // as well as the spaced spelling below because it is the one that is
            // a single whitespace-token: without it, a version invented by the
            // wrong-shape guard being *absent* is caught only by the bare-form
            // line-count check, which is a different rule.
            "channel=\"stable\"\n",
            "channel = \"stable\"\n",
            "",
        ] {
            let read = toolchain_from_text("rust-toolchain.toml", text);
            assert!(
                matches!(
                    read,
                    Err(UnreadReason::NotParsed { .. } | UnreadReason::WrongShape { .. })
                ),
                "{text:?} was read as a toolchain pin"
            );
        }

        // The same text under the older name, which is not defined as TOML: one
        // token is a channel, and more than one is not the file rustup
        // documents.
        assert_eq!(
            toolchain_from_text("rust-toolchain", "stable\n")
                .expect("one token is a channel")
                .channel
                .as_deref(),
            Some("stable")
        );
        for text in ["stable\n# a comment\n", "two words\n", "\n\n"] {
            assert!(
                matches!(
                    toolchain_from_text("rust-toolchain", text),
                    Err(UnreadReason::WrongShape { .. })
                ),
                "{text:?} was read as a channel"
            );
        }
    }

    #[test]
    fn a_toolchain_component_is_matched_without_regard_to_case() {
        let pinned = ToolchainState::Read(Box::new(
            toolchain_from_text(
                "rust-toolchain.toml",
                "[toolchain]\nchannel = \"stable\"\ncomponents = [\"Clippy\", \"RUSTFMT\"]\n",
            )
            .expect("a toolchain table"),
        ));
        assert!(pinned.asks_for("clippy"));
        assert!(pinned.asks_for("rustfmt"));
        assert!(!pinned.asks_for("miri"));
        assert!(!ToolchainState::Absent.asks_for("clippy"));
    }

    #[test]
    fn every_row_of_the_tool_table_names_a_crate_a_role_and_no_pair_twice() {
        let mut seen = BTreeSet::new();
        for &(package, role) in TOOLS {
            assert!(!package.is_empty());
            assert!(
                ToolRole::ALL.contains(&role),
                "{package} names a role that does not exist"
            );
            // The name is what is compared against a project's spelling, so a
            // row that is not lower case would silently never match a manifest
            // that wrote the crate's real name.
            assert_eq!(
                package,
                package.to_lowercase(),
                "{package} is not spelled the way a manifest spells it"
            );
            assert!(
                seen.insert((package, role)),
                "{package} names {role:?} twice"
            );
        }
        // Every role has to have at least one crate, or looking for it would be
        // work with no way to succeed.
        for &role in ToolRole::ALL {
            assert!(
                TOOLS.iter().any(|&(_, found)| found == role),
                "no crate names {role:?}"
            );
        }
    }

    #[test]
    fn what_a_finding_names_is_the_tables_own_crate_and_never_the_projects_spelling() {
        // A manifest can only write a crate name the parser accepted, so the
        // table's name and the project's are the same here. What the test holds
        // is the other direction: a crate not in the table produces no finding
        // at all, rather than a finding carrying the project's text.
        assert_eq!(
            declared_crates("[package]\nname = \"x\"\n\n[dependencies]\ntokio = \"1\"\n"),
            vec!["tokio"]
        );
        assert!(
            declared_crates(
                "[package]\nname = \"x\"\n\n[dependencies]\nsome-internal-crate = \"1\"\n"
            )
            .is_empty()
        );
        assert!(
            declared_crates("[package]\nname = \"x\"\n\n[dependencies]\nSerde = \"1\"\n")
                .is_empty(),
            "a crate name that is not the table's is not recognised"
        );
        // Dependencies are read where Cargo reads them: as siblings of
        // `[package]`, and not at all without one. A virtual manifest that
        // declares dependencies is one Cargo refuses, so reporting crates from
        // it would report a project Cargo does not have.
        assert!(
            declared_crates(
                "[workspace]\nmembers = [\"crates/*\"]\n\n[dependencies]\ntokio = \"1\"\n"
            )
            .is_empty(),
            "a manifest with no package table has no dependencies to read"
        );
    }

    #[test]
    fn a_benchmark_is_reachable_by_either_kind_of_evidence_and_by_both_together() {
        // `cargo bench` is planned on two independent routes: a `[[bench]]`
        // target declares a benchmark to run, and a benchmarking crate in the
        // dependencies is the harness that runs one. Either alone is enough,
        // because a project that declares only a harness has benchmarks the
        // harness would run, and a project with only a declared target has a
        // benchmark with no harness named.
        let by_target = project_of(
            r#"
            [package]
            name = "app"
            version = "0.1.0"

            [[bench]]
            name = "throughput"
            "#,
        );
        assert_eq!(
            command(&by_target, CommandRole::Bench).as_deref(),
            Some("cargo bench")
        );

        let by_dependency = project_of(
            r#"
            [package]
            name = "app"
            version = "0.1.0"

            [dependencies]
            criterion = "0.5"
            "#,
        );
        assert_eq!(
            command(&by_dependency, CommandRole::Bench).as_deref(),
            Some("cargo bench")
        );

        // Neither, and the command is not planned: `cargo bench` with nothing to
        // bench succeeds and does nothing, which is the shape of answer this
        // product exists not to give.
        let by_nothing = project_of(
            r#"
            [package]
            name = "app"
            version = "0.1.0"

            [dependencies]
            tokio = "1"
            "#,
        );
        assert_eq!(command(&by_nothing, CommandRole::Bench), None);
    }

    #[test]
    fn every_tool_role_can_be_named_and_described() {
        let roles: BTreeSet<&str> = ToolRole::ALL.iter().map(|role| role.as_str()).collect();
        assert_eq!(
            roles.len(),
            ToolRole::ALL.len(),
            "two roles answer to one name"
        );
        for role in ToolRole::ALL {
            assert!(!role.plain_name().is_empty());
        }
    }

    #[test]
    fn lints_are_read_for_the_two_tools_cargo_defines_and_a_third_is_not_named() {
        let package = project_of(
            r#"
            [package]
            name = "x"

            [lints.rust]
            unsafe_code = "forbid"

            [lints.clippy]
            unwrap_used = "warn"
            expect_used = "warn"

            [lints.somethingelse]
            whatever = "warn"
            "#,
        );
        let lints = &package.package().expect("a package").lints;
        let named: Vec<(&str, &str)> = lints
            .iter()
            .map(|setting| (setting.tool, setting.name.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                ("clippy", "expect_used"),
                ("clippy", "unwrap_used"),
                ("rust", "unsafe_code"),
            ]
        );
        assert_eq!(
            package
                .package()
                .expect("a package")
                .lints_of("clippy")
                .count(),
            2
        );
    }

    #[test]
    fn a_row_exists_for_every_conventional_role_whether_or_not_it_can_run() {
        let project = project_of("[package]\nname = \"x\"\n");
        let rows = project.conventional_commands();
        assert_eq!(rows.len(), CommandRole::ALL.len());
        for (row, &role) in rows.iter().zip(CommandRole::ALL) {
            assert_eq!(row.role, role, "the rows are in CommandRole::ALL order");
            assert!(!role.as_str().is_empty());
            assert!(!role.plain_name().is_empty());
            assert!(!role.tool().is_empty());
        }
    }

    #[test]
    fn a_command_is_only_offered_for_a_role_the_project_declared_a_tool_for() {
        // A plain crate with a manifest and nothing else: cargo's own roles are
        // planned, and the two that need a separate install are not.
        let plain = project_of("[package]\nname = \"x\"\n");
        assert_eq!(
            command(&plain, CommandRole::Build).as_deref(),
            Some("cargo build")
        );
        assert_eq!(
            command(&plain, CommandRole::Test).as_deref(),
            Some("cargo test")
        );
        assert_eq!(
            command(&plain, CommandRole::Check).as_deref(),
            Some("cargo check --all-targets")
        );
        assert_eq!(command(&plain, CommandRole::Lint), None);
        assert_eq!(command(&plain, CommandRole::Format), None);

        // The same crate with a linter configuration: clippy is now something
        // the project asked for.
        let mut configured = project_of("[package]\nname = \"x\"\n");
        configured.linter_config = Some("clippy.toml");
        assert_eq!(
            command(&configured, CommandRole::Lint).as_deref(),
            Some("cargo clippy --all-targets")
        );

        // And with rustfmt as a toolchain component rather than a file.
        let mut pinned = project_of("[package]\nname = \"x\"\n");
        pinned.toolchain = ToolchainState::Read(Box::new(
            toolchain_from_text(
                "rust-toolchain.toml",
                "[toolchain]\nchannel = \"stable\"\ncomponents = [\"rustfmt\"]\n",
            )
            .expect("a toolchain table"),
        ));
        assert_eq!(
            command(&pinned, CommandRole::Format).as_deref(),
            Some("cargo fmt")
        );
        assert_eq!(
            command(&pinned, CommandRole::Lint),
            None,
            "rustfmt is not clippy"
        );
    }

    #[test]
    fn a_row_that_has_no_command_carries_no_reason_for_one() {
        // A reason beside a command that is not there would say the project
        // declared something it did not.
        let mut project = project_of("[package]\nname = \"x\"\n");
        project.linter_config = Some("clippy.toml");
        for row in project.conventional_commands() {
            if row.command.is_none() {
                assert!(
                    row.because.is_empty(),
                    "{:?} has no command and {} reasons",
                    row.role,
                    row.because.len()
                );
                assert!(!row.is_planned());
            } else {
                assert!(row.is_planned());
            }
        }
        // The one that does have a command has a reason naming the file.
        let lint = project
            .conventional_commands()
            .into_iter()
            .find(|row| row.role == CommandRole::Lint)
            .expect("lint has a row");
        assert_eq!(lint.because.len(), 1);
        assert_eq!(lint.because[0].path, PathBuf::from("clippy.toml"));
    }

    #[test]
    fn a_command_names_a_constant_and_never_anything_a_project_wrote() {
        // Two projects that differ only in what they depend on. If any crate
        // name could reach a command, these would differ.
        let bare = project_of("[package]\nname = \"x\"\n");
        let loaded = project_of(
            r#"
            [package]
            name = "x"

            [dependencies]
            axum = "0.7"
            tokio = { version = "1", features = ["full"] }

            [dev-dependencies]
            criterion = "0.5"
            "#,
        );
        let commands_of = |project: &RustProject| -> Vec<Option<String>> {
            project
                .conventional_commands()
                .into_iter()
                .map(|row| row.command)
                .collect()
        };
        // `bench` is the one that legitimately differs, and it differs because a
        // benchmark runner is something the project declared — not because a
        // crate name reached the text.
        let bench = |project: &RustProject| {
            project
                .conventional_commands()
                .into_iter()
                .find(|row| row.role == CommandRole::Bench)
                .expect("bench has a row")
                .command
        };
        assert_eq!(bench(&bare), None);
        assert_eq!(bench(&loaded).as_deref(), Some("cargo bench"));
        assert!(loaded.declares("axum"));
        assert!(loaded.declares("tokio"));
        let other_roles = |project: &RustProject| -> Vec<Option<String>> {
            project
                .conventional_commands()
                .into_iter()
                .filter(|row| row.role != CommandRole::Bench)
                .map(|row| row.command)
                .collect()
        };
        assert_eq!(other_roles(&bare), other_roles(&loaded));
        assert_eq!(commands_of(&bare).len(), CommandRole::ALL.len());
    }

    #[test]
    fn cargo_run_is_planned_only_where_there_is_a_program_to_run() {
        // A library crate has no binary and `cargo run` there fails, so a plan
        // that offered it would be a plan about a different project.
        let library = project_of("[package]\nname = \"x\"\n");
        assert_eq!(command(&library, CommandRole::Run), None);

        let mut with_main = project_of("[package]\nname = \"x\"\n");
        with_main.conventional_targets.push(ConventionalTarget {
            path: PathBuf::from("src/main.rs"),
            kind: TargetKind::Binary,
        });
        assert_eq!(
            command(&with_main, CommandRole::Run).as_deref(),
            Some("cargo run")
        );

        // A virtual manifest has no package to run, even when a member has one.
        let virtual_manifest = project_of("[workspace]\nmembers = [\"crates/*\"]\n");
        assert_eq!(virtual_manifest.package(), None);
        assert_eq!(command(&virtual_manifest, CommandRole::Run), None);
        // ...but it is still a project cargo can build, and that is the whole
        // difference between a virtual manifest and a manifest SURE could not
        // read.
        assert_eq!(
            command(&virtual_manifest, CommandRole::Build).as_deref(),
            Some("cargo build")
        );
    }

    #[test]
    fn cargo_bench_is_planned_only_where_there_is_a_benchmark_to_run() {
        let bare = project_of("[package]\nname = \"x\"\n");
        assert_eq!(command(&bare, CommandRole::Bench), None);

        let mut with_benches = project_of("[package]\nname = \"x\"\n");
        with_benches.conventional_targets.push(ConventionalTarget {
            path: PathBuf::from("benches"),
            kind: TargetKind::Benchmark,
        });
        assert_eq!(
            command(&with_benches, CommandRole::Bench).as_deref(),
            Some("cargo bench")
        );

        let with_runner = project_of(
            r#"
            [package]
            name = "x"

            [dev-dependencies]
            criterion = "0.5"
            "#,
        );
        assert_eq!(
            command(&with_runner, CommandRole::Bench).as_deref(),
            Some("cargo bench")
        );
    }

    #[test]
    fn a_command_is_planned_only_for_a_manifest_sure_read() {
        // The three states, and the commands that follow from each. An unread
        // manifest must not produce the same plan as an absent one *and* must
        // not produce a plan at all: a command chosen for a project SURE could
        // not read is a plan about a different project.
        for state in [
            ManifestState::Absent,
            ManifestState::Unread(UnreadReason::NotParsed {
                detail: "fixture".to_owned(),
            }),
        ] {
            let project = RustProject {
                manifest: state,
                workspaces: Workspaces::default(),
                lockfile: false,
                toolchain: ToolchainState::Absent,
                formatter_config: None,
                linter_config: None,
                tooling: Vec::new(),
                conventional_targets: Vec::new(),
            };
            for row in project.conventional_commands() {
                assert_eq!(
                    row.command, None,
                    "{:?} was planned for a manifest SURE did not read",
                    row.role
                );
                assert!(row.because.is_empty());
            }
            assert_eq!(project.package(), None);
        }
    }

    #[test]
    fn a_member_target_makes_a_role_available_to_the_workspace() {
        // `cargo bench` at a virtual workspace's root runs the members'
        // benchmarks, so a bench target in a member is a benchmark the workspace
        // can run.
        let mut project = project_of("[workspace]\nmembers = [\"crates/a\"]\n");
        assert_eq!(command(&project, CommandRole::Bench), None);
        let mut member = PackageSection {
            name: Some("a".to_owned()),
            ..PackageSection::default()
        };
        member.targets.push(Target {
            kind: TargetKind::Benchmark,
            name: Some("throughput".to_owned()),
            declared_at: "bench",
        });
        project.workspaces.members.push(Member {
            path: PathBuf::from("crates/a"),
            manifest: MemberManifest::Present,
            package: Some(Box::new(member)),
        });
        assert_eq!(
            command(&project, CommandRole::Bench).as_deref(),
            Some("cargo bench")
        );
    }

    #[test]
    fn the_level_and_the_reason_are_decided_in_one_place() {
        let read = ManifestState::Read(Box::default());
        for (manifest, lockfile, toolchain) in [
            (read.clone(), false, false),
            (read.clone(), true, false),
            (read.clone(), false, true),
            (ManifestState::Absent, false, false),
            (ManifestState::Absent, true, false),
            (ManifestState::Absent, false, true),
            (ManifestState::Absent, true, true),
            (
                ManifestState::Unread(UnreadReason::TooLarge { limit: 1 }),
                true,
                true,
            ),
        ] {
            let declared_a_workspace = false;
            let (level, reason) = grade(&manifest, declared_a_workspace, lockfile, toolchain);
            assert!(!reason.is_empty());
            let expected = if matches!(manifest, ManifestState::Read(_)) {
                SupportLevel::Generic
            } else {
                SupportLevel::InspectOnly
            };
            assert_eq!(level, expected, "for {manifest:?}");
        }
        // A workspace on its own, with no manifest SURE read, is still only
        // inspect-only *here* — `grade` is reached with `Absent` only when the
        // manifest is gone, and a declared workspace is then a reason the level
        // is not "nothing here says this is Rust".
        let (_, reason) = grade(&ManifestState::Absent, true, false, false);
        assert!(reason.contains("says this is a Rust project"));
    }

    #[test]
    fn every_conventional_target_path_is_a_relative_path_with_a_known_shape() {
        let mut seen = BTreeSet::new();
        for &(path, kind, _) in CONVENTIONAL_TARGETS {
            assert!(
                !path.starts_with('/') && !path.contains('\\'),
                "{path} is not a relative path written with forward slashes"
            );
            assert!(
                Path::new(path).is_relative(),
                "{path} is not relative, and discovery's paths are relative"
            );
            assert!(TargetKind::ALL.contains(&kind));
            assert!(
                seen.insert(path),
                "{path} is listed twice, so one path would be reported twice"
            );
        }
        // Every kind is either discovered at a conventional path or named by a
        // table, and the build script is the one that has no table.
        for &kind in TargetKind::ALL {
            let conventional = CONVENTIONAL_TARGETS
                .iter()
                .any(|&(_, found, _)| found == kind);
            assert!(
                conventional || kind.table().is_some(),
                "{kind:?} can neither be found at a conventional path nor named by a \
                 table, so nothing could ever report it"
            );
        }
        assert!(
            CONVENTIONAL_TARGETS
                .iter()
                .any(|&(_, kind, _)| kind == TargetKind::BuildScript),
            "the build script is the target this list exists to make visible"
        );
    }

    #[test]
    fn workspace_dependencies_are_recorded_as_names_and_nothing_is_merged() {
        // A member saying `serde = { workspace = true }` keeps its own entry with
        // `FromWorkspace`, and the workspace's table keeps the name. Joining them
        // would produce a value that could not say which file it came from.
        let manifest = manifest_of(
            r#"
            [workspace]
            members = ["crates/a"]

            [workspace.dependencies]
            serde = { version = "1", features = ["derive"] }
            tokio = "1"
            "#,
        );
        assert_eq!(manifest.workspace_tables.inherited, vec!["serde", "tokio"]);
    }

    #[test]
    fn a_manifest_that_is_a_table_with_neither_section_reads_as_neither() {
        // `[patch]` and `[profile]` are real Cargo tables that say nothing SURE
        // reports. A manifest holding only those is a manifest SURE read, with no
        // package and no workspace — which is a different fact from a manifest
        // that failed to parse.
        let manifest = manifest_of(
            r#"
            [profile.release]
            lto = true
            "#,
        );
        assert!(manifest.package.is_none());
        assert!(!manifest.workspace_tables.declared);
    }

    #[test]
    fn a_number_in_a_members_list_is_not_turned_into_a_directory_name() {
        // `[workspace] members = [1, "crates/a"]` is TOML Cargo would refuse and
        // a file SURE still has to read without inventing anything from it. The
        // number stringified would become a member called `1`, and a member
        // called `1` is not a fact this file states.
        let manifest = manifest_of("[workspace]\nmembers = [1, \"crates/a\"]\n");
        assert_eq!(manifest.workspace_tables.patterns, vec!["crates/a"]);

        // The same rule for the other two lists, which go through the same
        // helper.
        let manifest = manifest_of("[workspace]\nexclude = [true]\ndefault-members = [[\"a\"]]\n");
        assert!(manifest.workspace_tables.excluded.is_empty());
        assert!(manifest.workspace_tables.default_members.is_empty());
    }

    #[test]
    fn a_value_is_read_without_the_whitespace_around_it() {
        // TOML keeps the spaces inside a string, and a name with a trailing
        // space is not a different name. What a finding carries is the value,
        // not the padding somebody's editor left in the file.
        let manifest = manifest_of(
            r#"
            [package]
            name = "  app  "
            version = " 1.0.0 "
            edition = "\t2024\n"
            rust-version = " 1.85 "
            "#,
        );
        let package = manifest.package.as_ref().expect("a package table");
        assert_eq!(package.name.as_deref(), Some("app"));
        assert_eq!(package.version.as_deref(), Some("1.0.0"));
        assert_eq!(package.edition.as_deref(), Some("2024"));
        assert_eq!(package.rust_version.as_deref(), Some("1.85"));

        // A value that is nothing but whitespace is nothing rather than an empty
        // string: `name = "   "` states no name, not a blank one.
        let manifest = manifest_of("[package]\nname = \"   \"\n");
        assert_eq!(
            manifest
                .package
                .as_ref()
                .expect("a package table")
                .name
                .as_deref(),
            None
        );
    }
}

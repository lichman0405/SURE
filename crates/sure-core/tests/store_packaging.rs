//! The two build-time promises the store makes, checked rather than assumed.
//!
//! `sure_core::store`'s module documentation states both, and both are the kind
//! of claim that quietly stops being true: a dependency feature is removed by
//! somebody tidying a manifest, and an async runtime arrives with a crate three
//! levels down. Neither shows up as a failing test or a wrong answer — the first
//! shows up as an installation that does not start on a machine without
//! `sqlite3.dll`, and the second as a rule the store's own documentation says it
//! keeps.
//!
//! So they are asserted here, against the manifests, which is where they are
//! decided.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

/// The checkout root, from this crate's compiled location.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the checkout root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every manifest in the workspace, workspace root first.
fn manifests() -> Vec<PathBuf> {
    let mut found = vec![root().join("Cargo.toml")];
    let crates = root().join("crates");
    let mut entries: Vec<_> = fs::read_dir(&crates)
        .unwrap_or_else(|error| panic!("{}: {error}", crates.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    entries.sort();
    found.extend(entries.into_iter().map(|path| path.join("Cargo.toml")));
    found
}

/// The text of the `[dependencies]` entry for `name` in a manifest.
///
/// Line-based rather than a real TOML parse. `sure-core` has no TOML
/// dependency of its own — `serde_yaml_ng` reads the configuration documents —
/// and adding one so a test can read a manifest would put a parser in the
/// shipped binary for the sake of the test.
fn dependency_entry(manifest: &str, name: &str) -> Option<String> {
    let mut found = None;
    let mut in_dependencies = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]" || trimmed == "[workspace.dependencies]";
            continue;
        }
        if !in_dependencies || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(name)
            && rest.trim_start().starts_with('=')
        {
            found = Some(rest.to_owned());
        }
    }
    found
}

#[test]
fn sqlite_is_compiled_into_sure_rather_than_looked_for_on_the_machine() {
    // The acceptance criterion "storage packages without requiring system
    // SQLite", at the level where it is decided. `bundled` compiles SQLite from
    // C source into the binary; without it `libsqlite3-sys` links a system
    // library, and `sure.exe` stops starting on a machine that has none.
    //
    // Checked here rather than left to the build, because the build does not
    // catch it. Removing `bundled` on this machine — which has no system
    // SQLite — still compiles: `cargo build -p sure-core` produces an rlib, and
    // an rlib is never linked. The failure arrives later, as
    // `LNK1181: cannot open input file 'sqlite3.lib'`, when something that
    // *does* link is built, and on a machine that happens to have the library
    // it never arrives at all — the installation just starts depending on it.
    let workspace = read(&root().join("Cargo.toml"));
    let entry = dependency_entry(&workspace, "rusqlite")
        .expect("rusqlite is not a workspace dependency, so nothing pins this");
    assert!(
        entry.contains("bundled"),
        "the rusqlite dependency no longer builds SQLite in: {entry}"
    );

    // And the crate that uses it inherits the workspace entry rather than
    // naming a version of its own, which is what makes the line above the one
    // that decides.
    let core = read(&root().join("crates/sure-core/Cargo.toml"));
    let entry = dependency_entry(&core, "rusqlite").expect("sure-core does not depend on rusqlite");
    assert!(
        entry.contains("workspace = true"),
        "sure-core names its own rusqlite rather than inheriting the workspace one: {entry}"
    );
}

#[test]
fn sqlite_is_actually_linked_so_the_promise_is_not_about_a_manifest_only() {
    // The manifest says `bundled`; this says a SQLite got linked. The two
    // together are the claim. Checking only the manifest would pass on a
    // workspace where the dependency had been dropped from the build entirely.
    let version = rusqlite::version();
    assert!(
        version.starts_with('3'),
        "unexpected SQLite version {version}"
    );
    let number: Vec<u32> = version
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    assert!(
        number.first().copied().unwrap_or(0) >= 3 && number.get(1).copied().unwrap_or(0) >= 37,
        "SQLite {version} predates STRICT tables, which the schema uses"
    );
}

#[test]
fn no_async_runtime_has_arrived() {
    // `docs/architecture/RUST_DESIGN.md`: "do not hold synchronous DB resources
    // across `.await`". The store has no async API and no connection pool, and
    // the way that is kept is that the workspace has no runtime to write one
    // against. Discipline would not survive the first person who wanted one.
    const RUNTIMES: &[&str] = &[
        "tokio",
        "async-std",
        "smol",
        "async-global-executor",
        "monoio",
        "glommio",
    ];

    for manifest in manifests() {
        let text = read(&manifest);
        for runtime in RUNTIMES {
            assert!(
                dependency_entry(&text, runtime).is_none(),
                "{} depends on {runtime}. The store is synchronous, and an async runtime is \
                 how a synchronous resource ends up held across an await.",
                manifest.display()
            );
        }
    }
}

#[test]
fn the_workspace_lints_forbid_unsafe_code() {
    // The store runs C compiled by `cc` and calls into it. That C is not SURE's
    // to audit; SURE's own code is, and the blanket ban is what keeps the
    // boundary where this crate can see it rather than spread through the
    // storage layer.
    let workspace = read(&root().join("Cargo.toml"));
    assert!(
        workspace.contains("unsafe_code"),
        "the workspace no longer states a policy on unsafe code"
    );
    assert!(
        workspace.contains("unsafe_code = \"forbid\""),
        "unsafe code is no longer forbidden workspace-wide"
    );
}

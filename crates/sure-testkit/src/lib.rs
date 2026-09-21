#![forbid(unsafe_code)]
//! Test support for SURE.
//!
//! This crate holds the things tests need and the product must not ship:
//! deterministic fixtures, fake harness events, fake model providers, golden
//! report assertions, scratch directories, and the checks that hold the
//! repository's own shape in place.
//!
//! Nothing in the product depends on this crate. `sure_testkit::workspace`
//! enforces that, which is the only way the rule survives contact with a
//! convenient `use` statement.

pub mod integrations;
pub mod program;
pub mod scratch;
pub mod source_manifest;
pub mod workspace;

pub use integrations::{IntegrationFile, ThinnessFinding};
pub use program::{copy_program, write_program};
pub use workspace::{BoundaryPolicy, Crate, DepKind, Edge, Violation, Workspace};

/// The repository root this crate was built inside.
///
/// Derived from the compile-time manifest location rather than the working
/// directory, so a test gives the same answer whether it is run from the
/// repository root, from a crate directory, or from an IDE.
///
/// # Panics
///
/// Panics if the checkout does not look like SURE. That is a broken working
/// copy, not a product condition, and a test that quietly skipped its
/// assertions would be worse than one that stops.
#[must_use]
pub fn repository_root() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    match manifest_dir.parent().and_then(std::path::Path::parent) {
        Some(root) if root.join("Cargo.toml").is_file() && root.join("crates").is_dir() => {
            root.to_path_buf()
        }
        _ => unreachable!(
            "sure-testkit is not inside a SURE checkout: {}",
            manifest_dir.display()
        ),
    }
}

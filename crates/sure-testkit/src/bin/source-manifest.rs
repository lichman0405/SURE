//! Check, or regenerate, `SHA256SUMS.txt` — the manifest's own maintenance tool.
//!
//! ```text
//! cargo run -p sure-testkit --bin source-manifest              # check; exit 1 if anything disagrees
//! cargo run -p sure-testkit --bin source-manifest -- --write   # rewrite the recorded digests
//! ```
//!
//! Before `P15-T020` the only thing that could rebuild this file was a scratch
//! script under `target/tmp`, which is git-ignored — so the file's only
//! maintainer was a tool no reader of the repository could find, and nothing in
//! the tree read the file at all. This binary is that tool, committed, and
//! `crates/sure-testkit/tests/source_manifest.rs` is the reader.
//!
//! # It reads the index, not your working tree
//!
//! Every digest here is of the bytes **git will store in the commit** — the
//! index blob — and not of the bytes on disk. That is deliberate and it is the
//! decision `P15-T020` exists to take:
//!
//! - the index form is identical on Windows, macOS and Linux, because
//!   `.gitattributes`' `eol=crlf` is applied on *checkout* and never to the
//!   object, while the working-tree form differs by machine and by clone;
//! - measured at `edb2b00` over a fresh `git clone`, a working-tree reader
//!   reports **8 of the 195 listed paths stale** that are perfectly correct,
//!   and an index reader reports 2 — the file's own inconsistency, removed by
//!   regenerating it from the index in the same commit.
//!
//! The consequence for a contributor is the order of the three steps, and it is
//! the only thing about this tool that is easy to get wrong:
//!
//! ```text
//! git add <the files you changed>          # 1. the index must hold them first
//! cargo run -p sure-testkit --bin source-manifest -- --write   # 2. rewrite the digests
//! git add SHA256SUMS.txt                   # 3. the manifest is a file too
//! ```
//!
//! Regenerating before staging would hash the *previous* commit's bytes and
//! write digests that the very next `git add` invalidates. That is not a
//! footgun this tool can remove — the index is the object being described — so
//! it is stated here, in `CONTRIBUTING.md`, and in the check's own failure
//! message.
//!
//! # What it will not do
//!
//! It never adds a path and never removes one. `SHA256SUMS.txt` is a *curated*
//! selection over a subset of the tree, and deciding what belongs in that
//! selection is not a program's call; a tool that silently grew the file would
//! also hide the fact that the selection is a choice. It refuses to write while
//! any listed path is missing from the index, because a listed path that cannot
//! be hashed is a fact about the manifest rather than something to paper over.

use std::process::ExitCode;

use sure_testkit::source_manifest::{self, Finding, MANIFEST};

fn main() -> ExitCode {
    let write = std::env::args()
        .skip(1)
        .any(|argument| argument == "--write");
    match run(write) {
        Ok(code) => code,
        Err(message) => {
            // The check could not be performed, which is not the same result as
            // the check having been performed and passed.
            eprintln!("{MANIFEST}: cannot be checked — {message}");
            ExitCode::from(3)
        }
    }
}

fn run(write: bool) -> Result<ExitCode, String> {
    let root = sure_testkit::repository_root();
    let manifest = source_manifest::read(&root).map_err(|error| error.to_string())?;
    let paths: Vec<String> = manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let index = source_manifest::Index::read(&root, &paths).map_err(|error| error.to_string())?;
    let found = source_manifest::findings(&manifest, &index);

    let absent: Vec<&Finding> = found
        .iter()
        .filter(|finding| matches!(finding, Finding::NotInTheIndex { .. }))
        .collect();

    println!(
        "{MANIFEST}: {} listed, {} match the index, {} stale, {} not in the index",
        manifest.entries.len(),
        manifest.entries.len() - found.len(),
        found.len() - absent.len(),
        absent.len()
    );
    for finding in &found {
        println!("  {finding}");
    }

    if !write {
        if found.is_empty() {
            return Ok(ExitCode::SUCCESS);
        }
        println!(
            "\n(the manifest is out of date; stage what you changed and run \
             `cargo run -p sure-testkit --bin source-manifest -- --write`)"
        );
        return Ok(ExitCode::from(1));
    }

    if !absent.is_empty() {
        eprintln!(
            "refusing to write while a listed path is not in the index: a listed path that cannot \
             be hashed is a fact about the manifest, not something to paper over."
        );
        return Ok(ExitCode::from(2));
    }

    if found.is_empty() {
        println!("nothing to write");
        return Ok(ExitCode::SUCCESS);
    }

    let corrected = source_manifest::corrected(&manifest, &index);
    let text = source_manifest::render(&corrected);
    std::fs::write(root.join(MANIFEST), text).map_err(|error| error.to_string())?;
    println!(
        "\nwrote {} updated digest(s); trailing newline preserved: {}",
        found.len(),
        manifest.trailing_newline
    );
    println!("stage it: git add {MANIFEST}");
    Ok(ExitCode::SUCCESS)
}

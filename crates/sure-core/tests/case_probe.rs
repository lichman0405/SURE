// A probe, not a test. It is committed to a scratch `claude/**` branch for one
// CI run and then deleted; it is never part of the product tree.
//
// `Cargo.toml`'s `[workspace.lints.clippy]` sets `panic = "warn"` and the CI job
// runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`
// **before** its `cargo test` step, so without the allow below this file would
// fail clippy and the test would never run -- and a probe that never runs
// reports nothing while looking like it reported.
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]
//
// P15-T018's first clause: "on the macOS CI job, or any macOS machine this loop
// can drive, two spellings of one path (`src/EMAIL/SEND.rs` and
// `src/email/send.rs`) are put to the filesystem and what it answers is quoted
// with the command that asked." The macOS runner is the only macOS machine this
// loop can drive, and a `cargo test` step captures a passing test's stdout, so
// the answer is delivered by **failing** with the measurement as the panic
// message. It fails on every platform on purpose; the macOS leg is the reading.

use std::path::Path;

/// Writes to one spelling and reads back the other. If the read succeeds and
/// returns the same bytes, the two spellings are one file. If it fails with
/// `NotFound`, they are two. That is the volume's own answer, which is the thing
/// P15-T018 is about -- `#[cfg(windows)]` in `recheck_lifecycle::normalise_path`
/// is the operating system's name for it, and the task's title says to ask the
/// volume instead.
fn probe(label: &str, base: &Path) -> String {
    let upper_dir = base.join("src").join("EMAIL");
    let lower_dir = base.join("src").join("email");

    for dir in [&upper_dir, &lower_dir] {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return format!(
                "  {label} at {}: could not create {}: {e}",
                base.display(),
                dir.display()
            );
        }
    }

    let written = upper_dir.join("SEND.rs");
    let read_as = lower_dir.join("send.rs");
    let body = b"the-upper-one\n";
    if let Err(e) = std::fs::write(&written, body) {
        return format!("  {label}: could not write {}: {e}", written.display());
    }

    let read_back = std::fs::read(&read_as);
    let verdict: String = match &read_back {
        Ok(bytes) if bytes == body => "ONE FILE: the volume folds case".to_owned(),
        Ok(bytes) => format!(
            "SAME PATH, DIFFERENT BYTES: {:?}",
            String::from_utf8_lossy(bytes)
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            "TWO FILES: the volume keeps case".to_owned()
        }
        Err(e) => format!("the read failed with something else: {e}"),
    };

    // A second question, asked the same way: does the directory listing show one
    // entry or two? `read_dir` on `src/` answers with what the volume holds.
    let listing = std::fs::read_dir(base.join("src")).map_or_else(
        |e| format!("<unreadable: {e}>"),
        |entries| {
            let mut names: Vec<String> = entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names.join(",")
        },
    );

    format!(
        "  {label} at {}\n      wrote {} ({} bytes)\n      read {}: Ok={} -> {verdict}\n      entries under src/: [{listing}]",
        base.display(),
        written.display(),
        body.len(),
        read_as.display(),
        read_back.is_ok()
    )
}

/// Both probes run inside a directory of their own that is removed afterwards,
/// so the workspace is left as CI found it -- a probe that writes `src/` into
/// the crate root would make the tree dirty for every later step of the job.
fn in_a_directory_of_its_own(label: &str, parent: &Path, tag: &str) -> String {
    let base = parent.join(format!("sure-case-probe-{tag}"));
    let _ = std::fs::remove_dir_all(&base);
    let out = probe(label, &base);
    let _ = std::fs::remove_dir_all(&base);
    out
}

#[test]
fn probe_case_insensitivity() {
    let report = format!(
        "\nCASE PROBE\n  os: {}\n  arch: {}\n  family: {}\n{}\n{}\nEND CASE PROBE",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY,
        // The directory a checkout lives in on CI, which is the volume
        // P15-T018's premise is about.
        in_a_directory_of_its_own("workspace(cwd)", Path::new("."), "cwd"),
        // And the temporary directory, which may be a different volume.
        in_a_directory_of_its_own("temp_dir", &std::env::temp_dir(), "tmp"),
    );

    panic!("{report}");
}

//! The shape of `.github/workflows/ci.yml`, and what that shape is for.
//!
//! `P15-T010`'s acceptance is one sentence:
//!
//! > *Windows is mandatory primary job; macOS/Linux core jobs mandatory;
//! > fmt/check/clippy/test/evals included appropriately.*
//!
//! That sentence is four claims about a file, and a file is the thing this
//! machine can check. `ci.yml` already carried the substance of all four when
//! this file was written; what it did not carry was anything that would
//! *notice* one of them leaving. This repository has paid for that class of
//! blindness: `scripts/check-non-windows.mjs`'s header records that `ci` was red
//! for **75 consecutive runs** while every local gate was green, and the census
//! is item 99 of `progress/HANDOFF.md`.
//!
//! # What this file proves, and what it cannot
//!
//! It reads the workflow as text and asserts the four properties, each with an
//! edit that must turn it red. It proves **what the file says**. It proves
//! **nothing** about whether the workflow is valid YAML, whether GitHub Actions
//! would accept it, whether any run has happened, or whether a run that happened
//! passed. A file that says the right thing is not a run that did the right
//! thing, and the 75-run streak is exactly those two being confused; the run
//! record lives in `progress/HANDOFF.md`, and the runner is GitHub.
//! `grep -rn '\.github' --include='*.rs' crates/` found no test that read these
//! workflows at all before this one.
//!
//! # The reader, and why it is line-based
//!
//! `crates/sure-cli/tests/winget_manifest.rs` reads its YAML the same way — as
//! lines — for the same reason: a YAML parser would be a new dependency, and the
//! properties here are about text a person edits by hand. What keeps a line
//! reader honest is that it reports what it found. Every rule is written so that
//! a reader that came back empty produces *violations rather than silence*:
//! `a_workflow_that_is_not_there_is_not_a_pass` feeds it nothing and requires it
//! to complain, and `the_reader_sees_the_jobs_that_are_there` fails if the jobs,
//! the runners or the commands were read as absent. The one thing it
//! deliberately cannot read is a `run: |` block; a step that used one is
//! reported as a fault rather than skipped, so a multi-line command cannot hide
//! from these rules.
//!
//! # The rules, and the reading each one freezes
//!
//! 1. **The workflow runs by itself** — triggered by `push` and by
//!    `pull_request`. A job no trigger reaches is not a mandatory job.
//! 2. **Windows is a job of its own** — some job runs on `windows-latest` and
//!    on nothing else. This is the reading "primary job" is taken in:
//!    `docs/adr/0007-windows-primary-development.md` makes Windows the primary
//!    development environment, and the Windows-only checks (the bootstrap
//!    validation, the PowerShell host) are a job rather than a step of a matrix
//!    leg. A reader who disagrees with that reading should change this rule on
//!    purpose, not discover it afterwards.
//! 3. **The core jobs are mandatory on all three platforms** — the job that runs
//!    the workspace's tests runs them on `windows-latest`, `macos-latest` and
//!    `ubuntu-latest`, and none of the three may leave.
//! 4. **No job is conditional and nothing is allowed to fail** — no job-level
//!    `if:`, and no `continue-on-error` at any level. A step-level `if:` is not
//!    this rule's business: `ci.yml` uses one to apply the Linux `sysctl` knob
//!    only on Linux, and that is how a platform-specific step is written.
//! 5. **The four command families are run** — `fmt`, `check`, `clippy` and
//!    `test`, each carrying the fragments that make it a gate rather than a
//!    report, in one command per family.
//! 6. **The evals are reached by that test command** — it is a whole-workspace
//!    `cargo test`, and `crates/sure-core/tests/acceptance_report_runner.rs`
//!    names `evaluation/acceptance-manifest.json`, declares no case with
//!    `#[ignore]`, and the manifest declares cases. The evals are plain
//!    `#[test]`s, so running them *is* what "included appropriately" means here;
//!    the release *metrics* of `docs/product/PRODUCT_EVALS.md` are computed from
//!    that same report by `sure_core::release_gate` and read by
//!    `scripts/Build-Release.*`, which is `P15-T011`'s subject and
//!    `release-dry-run.yml`'s file.
//!
//! # Two things that look like gaps and are not
//!
//! `ci.yml` never runs `scripts/check-non-windows.mjs`, and that is correct
//! rather than an omission: the script cross-compiles clippy for Unix targets
//! *from Windows*, and this workflow's `macos-latest` and `ubuntu-latest` matrix
//! legs compile those `cfg` arms natively. Adding it here would be a redundant
//! job, and `scripts/check-non-windows.mjs` says so in its own header.
//! `release-dry-run.yml` is `workflow_dispatch`-only on purpose — its own header
//! and `docs/development/GITHUB_WORKFLOW.md` say the two workflows answer
//! different questions — so nothing here asserts anything about it.
//!
//! # Why the assertions live in `sure-testkit`
//!
//! This crate already holds the checks that keep the repository's own shape in
//! place (`repository_shape.rs`, `integration_thinness.rs`), it runs on all
//! three platforms — so this check runs in every matrix leg it is about — and
//! nothing in the product depends on it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// The workflow this file is about.
const WORKFLOW: &str = ".github/workflows/ci.yml";

/// The corpus the evals are read from, and the test target that runs it.
const MANIFEST: &str = "evaluation/acceptance-manifest.json";
const EVAL_RUNNER: &str = "crates/sure-core/tests/acceptance_report_runner.rs";

/// The three platforms the acceptance makes mandatory, as the labels spell them.
const PLATFORMS: [&str; 3] = ["windows-latest", "macos-latest", "ubuntu-latest"];

/// What each command family has to carry, and why the fragment is not decoration.
///
/// These are the spellings the rest of the repository already uses: the
/// supervisor's gate set runs `cargo clippy --workspace --all-targets
/// --all-features -- -D warnings` and `cargo test --workspace --all-features
/// --no-fail-fast`, and `scripts/check-non-windows.mjs` runs the same clippy
/// command for the other platform. A CI run that selected a different set of
/// features would be a second definition of "the checks" — see the argument in
/// `docs/development/GITHUB_WORKFLOW.md`.
const REQUIRED: &[(&str, &[(&str, &str)])] = &[
    (
        "cargo fmt",
        &[(
            "--check",
            "a `fmt` step without it rewrites the tree instead of failing the run",
        )],
    ),
    (
        "cargo check",
        &[
            (
                "--workspace",
                "so every member is compiled and not only the default member",
            ),
            (
                "--all-targets",
                "so the tests and benches are compiled as well as the libraries",
            ),
        ],
    ),
    (
        "cargo clippy",
        &[
            ("--workspace", "so every member is linted"),
            (
                "--all-targets",
                "so the lint reaches the integration targets, which is where the defect that caused the 75-run streak lived",
            ),
            (
                "--all-features",
                "so a feature-gated path is linted by the same command that lints the rest",
            ),
            (
                "-D warnings",
                "without it clippy reports warnings and the run stays green",
            ),
        ],
    ),
    (
        "cargo test",
        &[
            (
                "--workspace",
                "so every member's test targets run, which is where the evals are",
            ),
            (
                "--all-features",
                "so the tests behind a feature run under the same command",
            ),
            (
                "--no-fail-fast",
                "the first failing target otherwise hides every target after it, and the targets are where the platform-specific tests live",
            ),
        ],
    ),
];

// --- reading --------------------------------------------------------------

fn repository_root() -> PathBuf {
    sure_testkit::repository_root()
}

/// A tracked file of the checkout, with its line endings normalized.
///
/// `.gitattributes` declares `* text=auto eol=lf`, and this machine's checkout is
/// not the only place these files are read, so a comparison written against LF
/// normalizes rather than assumes — the same reason, and the same shape, as
/// `script_text` in `crates/sure-cli/tests/install_flow.rs`.
fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// One significant line: not blank and not a whole-line comment.
struct Line<'a> {
    /// 1-based, so a violation can name the line a reader would open.
    number: usize,
    /// The leading whitespace, in bytes. Indentation here is spaces; a tab would
    /// make this number wrong and the reader would report missing jobs rather
    /// than mis-reading them quietly.
    indent: usize,
    text: &'a str,
}

fn significant_lines(text: &str) -> Vec<Line<'_>> {
    text.lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let text = raw.trim();
            if text.is_empty() || text.starts_with('#') {
                return None;
            }
            Some(Line {
                number: index + 1,
                indent: raw.len() - raw.trim_start().len(),
                text,
            })
        })
        .collect()
}

/// A `key: value` pair, with a YAML list dash removed and both halves trimmed.
fn key_and_value(text: &str) -> (&str, &str) {
    let text = text.strip_prefix("- ").unwrap_or(text);
    match text.split_once(':') {
        Some((key, value)) => (key.trim(), value.trim()),
        None => (text, ""),
    }
}

/// The bare labels in a value: an inline list, a scalar, or nothing at all.
///
/// A `${{ ... }}` expression yields no labels: the matrix job's
/// `runs-on: ${{ matrix.os }}` says *whatever the matrix says*, and reading it
/// as a label would be inventing one.
fn labels_in(value: &str) -> Vec<String> {
    let value = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(value);
    value
        .split(',')
        .map(|part| part.trim().trim_matches(|c| c == '\'' || c == '"'))
        .filter(|part| !part.is_empty() && !part.contains("${{"))
        .map(str::to_owned)
        .collect()
}

/// The keys of the workflow's top-level `on:` section.
///
/// Both spellings are read — a block (`on:` then `push:` lines) and an inline
/// list (`on: [push, pull_request]`) — because a formatting change should not
/// turn "the workflow has no triggers" into a true-looking report.
fn triggers(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut inside = false;
    for line in significant_lines(text) {
        if line.indent == 0 {
            inside = line.text.starts_with("on:");
            if inside {
                let (_, value) = key_and_value(line.text);
                if !value.is_empty() {
                    found.extend(labels_in(value));
                }
            }
            continue;
        }
        if inside {
            let (key, _) = key_and_value(line.text);
            if !key.is_empty() && !found.iter().any(|seen| seen == key) {
                found.push(key.to_owned());
            }
        }
    }
    found
}

/// A job, as this file reads it.
struct Job {
    /// The job id, as it is spelled under `jobs:`.
    name: String,
    /// The line the job id is on.
    line: usize,
    /// The literal runner labels: `runs-on:` plus the matrix's `os:` list.
    labels: Vec<String>,
    /// The `runs-on:` value as written, for a violation to quote.
    runs_on: Option<String>,
    /// Every single-line `run:` command, in the order the job declares them.
    commands: Vec<String>,
    /// Why this job is not mandatory, as sentences.
    faults: Vec<String>,
}

impl Job {
    fn new(name: &str, line: usize) -> Self {
        Self {
            name: name.to_owned(),
            line,
            labels: Vec::new(),
            runs_on: None,
            commands: Vec::new(),
            faults: Vec::new(),
        }
    }
}

/// Read every job out of the workflow text.
///
/// The structure it expects is the one the file is written in: job ids at two
/// spaces under a top-level `jobs:`, job keys at four, steps and matrix entries
/// deeper than that. `the_reader_sees_the_jobs_that_are_there` is what stops a
/// shape this reader cannot follow from reading as a clean workflow.
fn jobs(text: &str) -> Vec<Job> {
    let mut found: Vec<Job> = Vec::new();
    let mut job: Option<Job> = None;
    // The indent of the `matrix:` key of the job being read, when it has one.
    let mut matrix: Option<usize> = None;
    // Set while a matrix `os:` list written one label per line is being read.
    let mut collecting_os = false;
    // Only the `jobs:` section holds jobs. Without this, the `on:` section's
    // `push:` and `pull_request:` keys read as two jobs with no runner — which
    // is what this reader did until `the_reader_sees_the_jobs_that_are_there`
    // reported them by name.
    let mut in_jobs = false;

    for line in significant_lines(text) {
        if line.indent == 0 {
            in_jobs = line.text.starts_with("jobs:");
            if !in_jobs {
                found.extend(job.take());
            }
            continue;
        }
        if !in_jobs {
            continue;
        }
        if line.indent == 2 && line.text.ends_with(':') {
            found.extend(job.take());
            let name = line.text.trim_end_matches(':');
            matrix = None;
            collecting_os = false;
            job = Some(Job::new(name, line.number));
            continue;
        }
        let Some(current) = job.as_mut() else {
            continue;
        };

        if collecting_os {
            if let Some(label) = line.text.strip_prefix("- ") {
                current.labels.extend(labels_in(label));
                continue;
            }
            collecting_os = false;
        }

        let (key, value) = key_and_value(line.text);
        if line.indent == 4 {
            // A job-level key. A `run:` here would not be a step, and a
            // job-level `if:` is what makes a job conditional.
            match key {
                "runs-on" => {
                    current.labels.extend(labels_in(value));
                    current.runs_on = Some(value.to_owned());
                }
                "if" => current.faults.push(format!(
                    "the job `{}` (line {}) is conditional (`if: {value}`), so a run in which the \
                     condition is false reports nothing about it",
                    current.name, line.number
                )),
                "continue-on-error" => current.faults.push(format!(
                    "the job `{}` (line {}) carries `continue-on-error: {value}`, so a failing job \
                     would not be a failing run",
                    current.name, line.number
                )),
                _ => {}
            }
            continue;
        }

        // Everything deeper than a job key: steps, and the strategy's matrix.
        match key {
            "matrix" => {
                matrix = Some(line.indent);
                collecting_os = false;
            }
            // A matrix entry, and only a matrix entry: `if: matrix.os ==
            // 'ubuntu-latest'` is not one (it starts with `if:`), and neither is
            // a `- run:` line — which is what keeps this reader from finding
            // `ubuntu-latest` in a step and reporting a platform the matrix no
            // longer carries.
            "os" if matrix.is_some_and(|indent| line.indent == indent + 2) => {
                if value.is_empty() {
                    collecting_os = true;
                } else {
                    current.labels.extend(labels_in(value));
                }
            }
            "run" => {
                if value.is_empty() || value.starts_with('|') || value.starts_with('>') {
                    current.faults.push(format!(
                        "the step at line {} of job `{}` runs a multi-line shell (`run: {value}`), \
                         and this reader reads single-line commands only, so what that step runs is \
                         not covered by any rule here",
                        line.number, current.name
                    ));
                } else {
                    current.commands.push(value.to_owned());
                }
            }
            "continue-on-error" => current.faults.push(format!(
                "the step at line {} of job `{}` carries `continue-on-error: {value}`, so a failing \
                 step in a green job",
                line.number, current.name
            )),
            _ => {}
        }
    }
    found.extend(job.take());
    found
}

/// The jobs, as a phrase a violation can carry.
fn listing(jobs: &[Job]) -> String {
    if jobs.is_empty() {
        return "no job was read at all".to_owned();
    }
    jobs.iter()
        .map(|job| format!("`{}` on {:?}", job.name, job.labels))
        .collect::<Vec<_>>()
        .join(", ")
}

// --- the rules ------------------------------------------------------------

/// Every rule, applied to the workflow text. Empty means this file is content.
fn violations(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    let found_triggers = triggers(text);
    for wanted in ["push", "pull_request"] {
        if !found_triggers.iter().any(|trigger| trigger == wanted) {
            out.push(format!(
                "{WORKFLOW} is not triggered by `{wanted}`, so nothing runs its jobs automatically \
                 and a job no trigger reaches is not a mandatory job; the triggers read are \
                 {found_triggers:?}"
            ));
        }
    }

    let jobs = jobs(text);

    // Rule 2: Windows alone, which is the reading of "primary job" this file
    // takes.
    let windows_only = jobs.iter().any(|job| {
        let windows = job.labels.iter().any(|label| label == "windows-latest");
        let elsewhere = job.labels.iter().any(|label| label != "windows-latest");
        windows && !elsewhere
    });
    if !windows_only {
        out.push(format!(
            "no job runs on Windows and nothing else, so the checks only a Windows machine can make \
             have no job of their own: {}",
            listing(&jobs)
        ));
    }

    // Rule 3: the core jobs are mandatory on all three platforms.
    let core: Vec<&Job> = jobs
        .iter()
        .filter(|job| {
            job.commands
                .iter()
                .any(|command| command.starts_with("cargo test --workspace"))
        })
        .collect();
    if core.is_empty() {
        out.push(format!(
            "no job runs `cargo test --workspace`, so the workspace's tests — including the \
             acceptance corpus's evals — are not run by this workflow: {}",
            listing(&jobs)
        ));
    } else if !core.iter().any(|job| {
        PLATFORMS
            .iter()
            .all(|platform| job.labels.iter().any(|label| label == platform))
    }) {
        for job in &core {
            let missing: Vec<&str> = PLATFORMS
                .iter()
                .copied()
                .filter(|platform| !job.labels.iter().any(|label| label == platform))
                .collect();
            out.push(format!(
                "the job `{}` (line {}) runs the workspace's tests on {:?} and not on {missing:?}, \
                 and the acceptance makes all three platforms mandatory",
                job.name, job.line, job.labels
            ));
        }
    }

    // Rule 4: nothing conditional, nothing tolerant.
    for job in &jobs {
        out.extend(job.faults.iter().cloned());
        if job.labels.is_empty() {
            out.push(format!(
                "the job `{}` (line {}) declares no runner this file can read (`runs-on: {}`), so \
                 where it runs is not something this file says",
                job.name,
                job.line,
                job.runs_on.as_deref().unwrap_or("nothing")
            ));
        }
    }

    // Rule 5: the four families, each in one command carrying every fragment.
    for (family, fragments) in REQUIRED {
        let commands: Vec<&String> = jobs
            .iter()
            .flat_map(|job| job.commands.iter())
            .filter(|command| command.starts_with(family))
            .collect();
        if commands.is_empty() {
            out.push(format!(
                "no job in {WORKFLOW} runs a `{family}` command, and the acceptance lists it among \
                 the checks that have to be there"
            ));
            continue;
        }
        if commands.iter().any(|command| {
            fragments
                .iter()
                .all(|(fragment, _)| command.contains(fragment))
        }) {
            continue;
        }
        for (fragment, why) in fragments.iter() {
            if !commands.iter().any(|command| command.contains(fragment)) {
                out.push(format!(
                    "no `{family}` command carries `{fragment}`, and {why}"
                ));
            }
        }
        // Every fragment is carried by some command of this family and no single
        // command carries all of them: the gate has been split apart, and no one
        // command is the check the acceptance asks for.
        if fragments
            .iter()
            .all(|(fragment, _)| commands.iter().any(|command| command.contains(fragment)))
        {
            out.push(format!(
                "the `{family}` fragments are spread over more than one command and no single \
                 command is the gate: {commands:?}"
            ));
        }
    }

    out.dedup();
    out
}

/// Rule 6, over the two files it is about.
///
/// Taken as text so that the ways this could go false can be fed to it: the
/// `#[ignore]` and the empty manifest below never touch the real files, and the
/// real files are read by the test that calls this.
fn eval_violations(runner: &str, manifest: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if !runner.contains(MANIFEST) {
        out.push(format!(
            "{EVAL_RUNNER} does not name {MANIFEST}, so the target `cargo test --workspace` runs is \
             not the corpus's runner and nothing here says the evals are included"
        ));
    }
    if runner.contains("#[ignore") {
        out.push(format!(
            "{EVAL_RUNNER} carries an `#[ignore]`, so the case behind it is skipped by `cargo test \
             --workspace` while the run stays green: a case a run passes over is the false green \
             this repository refuses"
        ));
    }
    match serde_json::from_str::<serde_json::Value>(manifest) {
        Ok(value) => {
            let cases = value["cases"].as_array().map_or(0, Vec::len);
            if cases == 0 {
                out.push(format!(
                    "{MANIFEST} declares no cases, so `cargo test --workspace` runs no evals"
                ));
            }
        }
        Err(error) => out.push(format!(
            "{MANIFEST} is not JSON this reader can read ({error}), so nothing here can say whether \
             the evals are included"
        )),
    }
    out
}

// --- the repository, against every rule -----------------------------------

#[test]
fn the_ci_workflow_satisfies_every_rule() {
    let text = read(WORKFLOW);
    let found = violations(&text);
    assert!(
        found.is_empty(),
        "{WORKFLOW} no longer carries what the acceptance asks of it:\n{}",
        found
            .iter()
            .map(|violation| format!("  - {violation}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_evals_are_reached_by_the_command_ci_runs() {
    let found = eval_violations(&read(EVAL_RUNNER), &read(MANIFEST));
    assert!(
        found.is_empty(),
        "the acceptance corpus is not included the way this file has to be able to say:\n{}",
        found
            .iter()
            .map(|violation| format!("  - {violation}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// --- and the reader and the rules, against edits that break them ----------

/// The reader found jobs, runners and commands, rather than an empty file.
///
/// Without this, a reader that returned nothing would make every "no job does
/// X" rule pass and every "no job is conditional" rule pass vacuously — the
/// failure mode of a checker that is green because it is blind.
#[test]
fn the_reader_sees_the_jobs_that_are_there() {
    let text = read(WORKFLOW);
    let jobs = jobs(&text);
    let names: Vec<&str> = jobs.iter().map(|job| job.name.as_str()).collect();
    for expected in ["bootstrap-validate-windows", "rust", "shellcheck-secondary"] {
        assert!(
            names.contains(&expected),
            "the reader did not find the job `{expected}`, and it read {names:?}"
        );
    }
    for job in &jobs {
        assert!(
            !job.labels.is_empty(),
            "the job `{}` (line {}) was read with no runner at all",
            job.name,
            job.line
        );
        assert!(
            !job.commands.is_empty(),
            "the job `{}` (line {}) was read with no commands at all",
            job.name,
            job.line
        );
    }
    let rust = jobs
        .iter()
        .find(|job| job.name == "rust")
        .expect("the core job is one of the jobs the loop above required");
    for platform in PLATFORMS {
        assert!(
            rust.labels.iter().any(|label| label == platform),
            "the reader did not read `{platform}` out of the matrix, and it read {:?}",
            rust.labels
        );
    }
    assert!(
        rust.commands
            .iter()
            .any(|command| command.starts_with("cargo test")),
        "the core job's commands were read as {:?}",
        rust.commands
    );
}

/// An empty workflow is not a pass, and neither is a job with no runner.
#[test]
fn a_workflow_that_is_not_there_is_not_a_pass() {
    let found = violations("");
    assert!(
        !found.is_empty(),
        "an empty file satisfied every rule, so the rules below prove nothing about a file that is \
         there"
    );
    // The same for a file that has the jobs and none of the runners: the
    // platform rules have to complain rather than read a platform out of
    // nothing.
    let no_runners = violations(
        "name: ci\non:\n  push:\njobs:\n  rust:\n    steps:\n      - run: cargo test --workspace\n",
    );
    assert!(
        no_runners
            .iter()
            .any(|violation| violation.contains("declares no runner")),
        "a job with no runner was read as a job with a runner: {no_runners:#?}"
    );
}

/// An edit to the real workflow that must turn exactly one rule red.
struct Break {
    /// What the edit is, in a sentence.
    what: &'static str,
    /// Text taken out of the workflow as it stands.
    from: &'static str,
    /// What goes in its place.
    to: &'static str,
    /// Text the resulting violation has to carry.
    wanted: &'static str,
}

/// Every rule, broken the way it would really be broken.
///
/// The edits are taken from the file's own bytes rather than written as
/// fixtures: a fixture proves the checker reacts to a workflow nobody has, and
/// an edit proves it reacts to this one. `replace` that found nothing is a
/// failure rather than a pass — a stale edit must redden rather than quietly
/// mutate nothing, which is the way this kind of table rots.
const BREAKS: &[Break] = &[
    Break {
        what: "the workflow stops running on pull requests",
        from: "  pull_request:",
        to: "  # pull_request:",
        wanted: "pull_request",
    },
    Break {
        what: "the Windows-only job is moved off Windows",
        from: "    runs-on: windows-latest",
        to: "    runs-on: ubuntu-latest",
        wanted: "on Windows and nothing else",
    },
    Break {
        what: "Windows leaves the matrix",
        from: "os: [windows-latest, macos-latest, ubuntu-latest]",
        to: "os: [macos-latest, ubuntu-latest]",
        wanted: "windows-latest",
    },
    Break {
        what: "macOS leaves the matrix",
        from: "os: [windows-latest, macos-latest, ubuntu-latest]",
        to: "os: [windows-latest, ubuntu-latest]",
        wanted: "macos-latest",
    },
    Break {
        what: "Linux leaves the matrix",
        from: "os: [windows-latest, macos-latest, ubuntu-latest]",
        to: "os: [windows-latest, macos-latest]",
        wanted: "ubuntu-latest",
    },
    Break {
        what: "a job is allowed to fail",
        from: "  rust:\n    strategy:",
        to: "  rust:\n    continue-on-error: true\n    strategy:",
        wanted: "continue-on-error",
    },
    Break {
        what: "a step is allowed to fail",
        from: "      - run: cargo fmt --all -- --check",
        to: "      - run: cargo fmt --all -- --check\n        continue-on-error: true",
        wanted: "continue-on-error",
    },
    Break {
        what: "a job becomes conditional",
        from: "  rust:\n    strategy:",
        to: "  rust:\n    if: github.event_name == 'push'\n    strategy:",
        wanted: "conditional",
    },
    Break {
        what: "the format check stops being run",
        from: "      - run: cargo fmt --all -- --check\n",
        to: "",
        wanted: "cargo fmt",
    },
    Break {
        what: "the compile check stops being run",
        from: "      - run: cargo check --workspace --all-targets\n",
        to: "",
        wanted: "cargo check",
    },
    Break {
        what: "clippy stops failing the run on warnings",
        from: "cargo clippy --workspace --all-targets --all-features -- -D warnings",
        to: "cargo clippy --workspace --all-targets --all-features",
        wanted: "-D warnings",
    },
    Break {
        what: "the test step stops disabling fail-fast",
        from: "cargo test --workspace --all-features --no-fail-fast",
        to: "cargo test --workspace --all-features",
        wanted: "--no-fail-fast",
    },
    Break {
        what: "the test step selects a different feature set from the gate",
        from: "cargo test --workspace --all-features --no-fail-fast",
        to: "cargo test --workspace --no-fail-fast",
        wanted: "--all-features",
    },
    Break {
        what: "the test step stops running the whole workspace",
        from: "cargo test --workspace --all-features --no-fail-fast",
        to: "cargo test -p sure-core --lib --all-features --no-fail-fast",
        wanted: "cargo test --workspace",
    },
    Break {
        what: "a step is written as a multi-line shell, where no rule here can see it",
        from: "      - run: cargo fmt --all -- --check",
        to: "      - run: |\n          cargo fmt --all -- --check",
        wanted: "multi-line shell",
    },
];

#[test]
fn every_rule_is_turned_red_by_an_edit_that_breaks_it() {
    let text = read(WORKFLOW);
    assert!(
        violations(&text).is_empty(),
        "the workflow is already failing a rule, so no edit below can be said to have broken it"
    );
    for edit in BREAKS {
        let broken = text.replace(edit.from, edit.to);
        assert_ne!(
            broken, text,
            "\"{}\" edits text that is not in {WORKFLOW} any more ({}), so it would have mutated \
             nothing and passed for that reason instead. The table has to be re-pointed at what the \
             file says now.",
            edit.what, edit.from
        );
        let found = violations(&broken);
        assert!(
            found
                .iter()
                .any(|violation| violation.contains(edit.wanted)),
            "breaking `{}` produced no violation carrying {:?}:\n{found:#?}",
            edit.what,
            edit.wanted
        );
    }
}

#[test]
fn the_eval_rule_is_turned_red_by_the_three_ways_it_could_be_false() {
    let runner = read(EVAL_RUNNER);
    let manifest = read(MANIFEST);

    // The real files pass, which is the only reason the negatives below mean
    // anything.
    assert!(eval_violations(&runner, &manifest).is_empty());

    // A case marked `#[ignore]` is a case `cargo test --workspace` skips.
    let ignored = format!("#[test]\n#[ignore = \"flaky\"]\nfn a_case() {{}}\n{runner}");
    assert!(
        eval_violations(&ignored, &manifest)
            .iter()
            .any(|violation| violation.contains("#[ignore]")),
        "an ignored case was not reported"
    );

    // A runner that stopped naming the corpus.
    let renamed = runner.replace(MANIFEST, "evaluation/somewhere-else.json");
    assert_ne!(renamed, runner, "the runner does not name {MANIFEST}");
    assert!(
        eval_violations(&renamed, &manifest)
            .iter()
            .any(|violation| violation.contains(MANIFEST)),
        "a runner that names no corpus was not reported"
    );

    // A manifest with no cases, and one that is not JSON at all.
    for empty in ["{\"schema_version\": 1, \"cases\": []}", "not json"] {
        assert!(
            !eval_violations(&runner, empty).is_empty(),
            "a corpus of {empty:?} was read as a corpus with cases in it"
        );
    }
}

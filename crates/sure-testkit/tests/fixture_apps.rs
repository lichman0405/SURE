//! The adversarial fixture applications, checked as artefacts.
//!
//! `P14-T001`'s acceptance is one sentence:
//!
//! > *Fake payment/auth/email/dead button/route/demo cases runnable with
//! > expected outcomes.*
//!
//! `P14-T002` added a second sentence for the same corpus:
//!
//! > *Missing config/migration/fake integration cases runnable.*
//!
//! `P14-T003` added a third, and a fourth:
//!
//! > *At least one Rust real-fail/real-pass project exercises
//! > discovery/execution/reporting.*
//!
//! `P14-T004` added a fifth:
//!
//! > *Tests-not-run/stale evidence/cannot-confirm outcomes exact.*
//!
//! The three fixtures that sentence names are not project directories at all:
//! each is a **recording** — a claim an AI made and the harness events behind it
//! — so "runnable" is not a question this file can ask about them and they are
//! not in any of the language lists below. [`CLAIM_FIXTURES`] names them, and
//! every check that is about a scenario rather than about a language applies to
//! them here exactly as it applies to a project: the schema, the false-green
//! rule, the module they say grades them, and the directory they ship in. What
//! their outcomes are is not here — that needs SURE's claim checker, and it
//! lives in `crates/sure-core/tests/adversarial_fixture_detection.rs`, where the
//! declared streams are read and the answers are asserted.
//!
//! Three words in each sentence are checkable without running SURE at all, and
//! this file checks them:
//!
//! - **runnable** — the fixture has a start command a reviewer with nothing
//!   installed can run, it names a file that exists, and it needs no package
//!   installed first. The Node half runs `node <file>`; the Python half runs
//!   `python <file>`, which is the interpreter name the fixtures are recorded
//!   against — `python3` is not on `PATH` on the machines this ships to, so
//!   an entry point that uses it is refused here rather than discovered by a
//!   reviewer whose shell answers with a Microsoft Store stub; the Rust half
//!   runs `cargo`, which every machine that builds this repository already has,
//!   from a PowerShell script, and its manifest declares nothing to fetch;
//! - **expected outcomes** — the fixture carries a `scenario.json` that parses
//!   and satisfies `schemas/fixture-expectation.schema.json`, and, where the
//!   release manifest has a case for it, that agrees with
//!   `evaluation/acceptance-manifest.json` about how serious the case is;
//! - **the false-green rule** — a release-blocking fixture says, in machine
//!   readable form, that "passing" is an outcome SURE must not produce.
//!
//! # The fixtures with no manifest case
//!
//! Three of these fixtures have no case in `evaluation/acceptance-manifest.json`,
//! and that file is the release contract rather than a place to add rows.
//! [`FIXTURES_WITHOUT_A_MANIFEST_CASE`] names each of them once, with the
//! reason, and `every_fixture_without_a_manifest_case_is_one_named_here` proves
//! the exemptions are still true — an entry there for a case the manifest *does*
//! have fails, so the list cannot silently widen and let a graded fixture out
//! of the severity agreement above.
//!
//! Two of the three are the `P14-T003` pair, and the reason they are exempt is
//! the same for both and worth stating here: the manifest's twenty cases do not
//! include any Rust or language-generic case, so there is no row whose severity
//! either half could be said to implement. Renaming a fixture to the nearest
//! case by name — `tests-not-run`, which is a project whose tests were never run
//! rather than one whose tests ran and failed — would make that row grade a
//! different defect from the one it was written for.
//!
//! What is deliberately **not** here: whether the defect is actually detected.
//! That claim needs SURE's scanners, which this crate cannot see, and it lives
//! in `crates/sure-core/tests/adversarial_fixture_detection.rs` where the
//! scanners are reachable. For the Rust pair it is stronger than a scan and
//! lives one crate closer: `crates/sure-core/tests/rust_fixture_apps.rs` runs
//! both halves through discovery, SURE's own process runner, the aggregation and
//! the verdict, and asserts that the two answers differ.
//!
//! # Why the checkers are functions rather than assertions inlined in the tests
//!
//! `repository_shape.rs` states the rule this file follows: *a check like this is
//! only worth having if it can fail.* Every checker below is therefore a plain
//! function returning the reasons it rejected a value, and every one of them is
//! fed a hand-built case that it **must** reject. A checker that returned no
//! violations for everything would fail those cases, so it cannot pass by being
//! vacuous. `the_python_runnable_checker_rejects_a_project_that_is_not_stdlib_only`
//! and `the_rust_runnable_checker_rejects_a_project_that_would_fetch_or_write`
//! are that rule held for two of the three languages.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The fixtures this task implemented, named rather than discovered.
///
/// A test that discovered them would pass on an empty directory, which is the
/// one way this file could quietly stop testing anything.
const TYPESCRIPT_FIXTURES: &[&str] = &[
    "fake-payment",
    "fake-auth",
    "fake-email",
    "dead-button",
    "demo-analytics",
    "route-mismatch",
];

/// The `P14-T002` fixtures, named for the same reason as the list above.
const PYTHON_FIXTURES: &[&str] = &["missing-migration", "external-unverified", "missing-config"];

/// The `P14-T003` fixtures: one Rust project whose own check fails, and the same
/// project with one line corrected.
///
/// The pair is one entry in this file's terms — the pass half is the failing
/// half's control, and the test that runs them is
/// `crates/sure-core/tests/rust_fixture_apps.rs`, where SURE's own discovery,
/// runner, aggregation and verdict are reachable.
const RUST_FIXTURES: &[&str] = &["rust-tests-fail", "rust-tests-pass"];

/// The `P14-T004` fixtures: an AI's claim and the recording behind it, with no
/// project in either.
///
/// They are named for the same reason as the lists above. They are deliberately
/// *not* chained into [`TYPESCRIPT_FIXTURES`] or its siblings even though they
/// are checked by the same scenario-wide tests: a list that meant "this fixture
/// is runnable with npm" cannot also mean "this fixture is a recording", because
/// the second would then be read as a claim about the first.
const CLAIM_FIXTURES: &[&str] = &["tests-not-run", "stale-test-evidence", "unknown-evidence"];

/// The fixtures that ship with no case in `evaluation/acceptance-manifest.json`,
/// and why each one does not have to.
///
/// An entry here is a claim that the release contract does not grade this
/// fixture, and `every_fixture_without_a_manifest_case_is_one_named_here`
/// checks the claim against the manifest rather than trusting it. Nothing else
/// in this file reads this list as an exemption: a fixture named here is still
/// checked for the schema, the false-green rule and runnability, and only the
/// severity agreement is waived — because there is no manifest row to agree
/// with.
const FIXTURES_WITHOUT_A_MANIFEST_CASE: &[(&str, &str)] = &[
    (
        "missing-config",
        "the manifest's case list is the release contract and P14-T002 was not asked to add a row to it; \
         the nearest case, `external-unverified`, is a different defect and is not claimed to cover this one",
    ),
    (
        "rust-tests-fail",
        "the manifest has no Rust case and no language-generic one, so no row's trap is this fixture's: \
         a Rust project whose own tests ran and failed. `tests-not-run`, the nearest by name, is the \
         opposite defect — a project whose tests were never run — and renaming this fixture after that \
         case would make its row grade something it was not written for",
    ),
    (
        "rust-tests-pass",
        "the same as `rust-tests-fail`, whose control this half is: it exists so that the failing half's \
         verdict is a measurement rather than a checker that refuses every Rust project, and the manifest \
         has no case for a project that is meant to come back green",
    ),
];

/// The marker the task graph leaves on a fixture that is still a stub.
const NOT_IMPLEMENTED: &str = "to_be_implemented_by_task_graph";

/// Every module a Python fixture may import.
///
/// Deliberately a list rather than a rule read out of the interpreter: the
/// fixtures are stdlib-only, and a name outside this list is refused so that
/// adding one is a decision someone makes here, in writing, instead of an
/// import that works on the machine it was written on. `app` is not here
/// because it is the fixtures' own package, and a file in the fixture is
/// treated as local — see [`python_imports`].
const ALLOWED_PYTHON_MODULES: &[&str] = &["dataclasses", "os", "pathlib", "sqlite3", "sys"];

/// The interpreter name the Python fixtures are recorded against.
///
/// `python3` is a Microsoft Store alias on the machines this project is
/// developed on: it opens the Store instead of running the file, so an entry
/// point spelled that way is not runnable there even though it reads as though
/// it is.
const PYTHON: &str = "python ";

/// The tool the Rust fixtures are recorded against, and the prefix every one of
/// their entry points has to carry.
///
/// `cargo` rather than a path to a toolchain: a fixture that pinned one would be
/// describing the machine it was written on. It is already required to build
/// this repository, so a reviewer has it.
const CARGO: &str = "cargo ";

fn root() -> PathBuf {
    sure_testkit::repository_root()
}

fn adversarial_dir() -> PathBuf {
    root().join("fixtures").join("adversarial")
}

fn fixture_dir(id: &str) -> PathBuf {
    adversarial_dir().join(id)
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()))
}

fn scenario_text(id: &str) -> String {
    let path = fixture_dir(id).join("scenario.json");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn scenario(id: &str) -> Value {
    read_json(&fixture_dir(id).join("scenario.json"))
}

fn manifest() -> Value {
    read_json(&root().join("evaluation").join("acceptance-manifest.json"))
}

fn expectation_schema() -> Value {
    read_json(
        &root()
            .join("schemas")
            .join("fixture-expectation.schema.json"),
    )
}

/// The manifest's entry for one case id, if it has one.
fn manifest_case(id: &str) -> Option<Value> {
    manifest()["cases"]
        .as_array()
        .expect("the manifest lists cases")
        .iter()
        .find(|case| case["id"].as_str() == Some(id))
        .cloned()
}

/// Every fixture directory this repository ships, by name.
fn shipped_fixture_ids() -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(adversarial_dir())
        .expect("fixtures/adversarial must be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect();
    ids.sort();
    ids
}

/// Every fixture the tasks named in this file implemented, both halves.
///
/// One list for the checks that are about a fixture rather than about a
/// language: the schema, the false-green rule, the check module and the
/// directory all apply to a Python fixture exactly as they apply to a Node one,
/// and to a recording that ships no project at all.
fn every_fixture_this_task_implemented() -> impl Iterator<Item = &'static str> {
    TYPESCRIPT_FIXTURES
        .iter()
        .chain(PYTHON_FIXTURES)
        .chain(RUST_FIXTURES)
        .chain(CLAIM_FIXTURES)
        .copied()
}

/// Why this fixture does not have to agree with the release manifest, if it
/// does not.
fn manifest_exemption(id: &str) -> Option<&'static str> {
    FIXTURES_WITHOUT_A_MANIFEST_CASE
        .iter()
        .find(|(exempt, _)| *exempt == id)
        .map(|(_, reason)| *reason)
}

// --- the checkers --------------------------------------------------------

/// Whether a JSON value has the type a schema names.
fn has_type(value: &Value, type_name: &str) -> bool {
    match type_name {
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "null" => value.is_null(),
        _ => false,
    }
}

/// Every reason a scenario document does not satisfy the expectation schema.
///
/// The schema is read rather than restated: a key it starts requiring, or a
/// type it starts refusing, turns up here without this file being edited.
fn schema_violations(document: &Value, schema: &Value) -> Vec<String> {
    let mut violations = Vec::new();

    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if required.is_empty() {
        violations.push("the schema requires nothing, so it validates anything".to_owned());
    }

    let properties = schema.get("properties").cloned().unwrap_or(Value::Null);

    for key in required.iter().filter_map(Value::as_str) {
        let Some(property) = properties.get(key) else {
            violations.push(format!("the schema requires `{key}` but declares no type"));
            continue;
        };
        let wants = property
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match document.get(key) {
            None => violations.push(format!("`{key}` is required and is missing")),
            Some(value) if !has_type(value, wants) => {
                violations.push(format!("`{key}` must be a {wants}"));
            }
            Some(value) => {
                // An array of objects is the shape both outcome lists have, and
                // an array of strings would satisfy `type: array` while making
                // every reader of it guess.
                let items_type = property
                    .get("items")
                    .and_then(|items| items.get("type"))
                    .and_then(Value::as_str);
                if wants == "array"
                    && let Some(items_type) = items_type
                    && let Some(items) = value.as_array()
                {
                    for (index, item) in items.iter().enumerate() {
                        if !has_type(item, items_type) {
                            violations.push(format!("`{key}[{index}]` must be a {items_type}"));
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Every reason a scenario does not forbid the false-green shape.
///
/// A release-blocking case that does not say "this must never be reported as
/// passing" has no rule for the eval runner to enforce, and the release rule
/// this product is built around — a false green is worse than a visible error —
/// would be unwritten for that case.
fn false_green_violations(document: &Value, release_blocking: bool) -> Vec<String> {
    let mut violations = Vec::new();
    let forbidden = document
        .get("forbidden_outcomes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if forbidden.is_empty() {
        violations.push("forbidden_outcomes is empty".to_owned());
        return violations;
    }

    let names_green = forbidden.iter().any(|outcome| {
        outcome
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind == "false_green")
    });
    if !names_green {
        violations.push("no forbidden outcome has kind `false_green`".to_owned());
    }

    if release_blocking {
        let mentions = |word: &str| {
            forbidden.iter().any(|outcome| {
                outcome
                    .get("description")
                    .and_then(Value::as_str)
                    .is_some_and(|text| text.to_lowercase().contains(word))
            })
        };
        if !mentions("green") {
            violations.push("no forbidden outcome says the project must not go green".to_owned());
        }
        if !mentions("passing") && !mentions("verified") && !mentions("working") {
            violations.push(
                "no forbidden outcome says the case must not be reported as passing".to_owned(),
            );
        }
        if forbidden.len() < 2 {
            violations
                .push("a release-blocking case needs more than one forbidden outcome".to_owned());
        }
    }

    violations
}

/// Every reason a fixture is not runnable by someone with only Node installed.
fn runnable_violations(dir: &Path) -> Vec<String> {
    let mut violations = Vec::new();
    let path = dir.join("package.json");
    if !path.is_file() {
        violations.push("there is no package.json".to_owned());
        return violations;
    }
    let package = read_json(&path);

    for key in ["dependencies", "devDependencies", "peerDependencies"] {
        let declared = package
            .get(key)
            .and_then(Value::as_object)
            .map(serde_json::Map::len)
            .unwrap_or(0);
        if declared > 0 {
            violations.push(format!(
                "package.json declares {declared} {key}, so the fixture is not runnable without npm install"
            ));
        }
    }

    let scripts = package.get("scripts").cloned().unwrap_or(Value::Null);
    let start = scripts.get("start").and_then(Value::as_str);
    let test = scripts.get("test").and_then(Value::as_str);

    if start.is_none() && test.is_none() {
        violations.push("package.json declares neither a start nor a test script".to_owned());
        return violations;
    }

    for (role, script) in [("start", start), ("test", test)] {
        let Some(script) = script else { continue };
        let Some(entry) = script.strip_prefix("node ") else {
            violations.push(format!(
                "the {role} script must run node directly, and it says `{script}`"
            ));
            continue;
        };
        let entry = entry.split_whitespace().next().unwrap_or_default();
        if !dir.join(entry).is_file() {
            violations.push(format!(
                "the {role} script names {entry}, which is not a file in the fixture"
            ));
        }
    }

    for source in javascript_files(dir) {
        for bare in bare_requires(&std::fs::read_to_string(&source).unwrap_or_default()) {
            violations.push(format!(
                "{} requires `{bare}`, which is not a Node builtin and is not in the fixture",
                source
                    .strip_prefix(dir)
                    .unwrap_or(source.as_path())
                    .display()
            ));
        }
    }

    violations
}

/// Every `.js` file under a directory, sorted, so a run is reproducible.
fn javascript_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "js") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The module names a source file requires that are neither builtins nor local.
///
/// `require('node:http')` and `require('./payments')` are fine. Anything else is
/// a package the reviewer would have to install first, which is the one thing
/// these fixtures may not need.
fn bare_requires(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for quote in ['\'', '"'] {
        let marker = format!("require({quote}");
        let mut search_from = 0;
        while let Some(start) = text[search_from..].find(&marker) {
            let after = search_from + start + marker.len();
            let Some(end) = text[after..].find(quote) else {
                break;
            };
            let name = &text[after..after + end];
            if !name.starts_with('.') && !name.starts_with("node:") {
                found.push(name.to_owned());
            }
            search_from = after + end + 1;
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Every reason a Python fixture is not runnable with nothing installed.
///
/// The Python half of [`runnable_violations`], and the differences are the
/// language's rather than this file's. There is no manifest that declares
/// dependencies in one place, so the imports are read out of the source and
/// compared against [`ALLOWED_PYTHON_MODULES`]; the entry points come out of
/// the fixture's `scenario.json`, which is the same document the reviewers and
/// the eval runner read, so the command checked here is the command they run.
fn python_runnable_violations(dir: &Path) -> Vec<String> {
    let mut violations = Vec::new();

    let package = dir.join("pyproject.toml");
    if !package.is_file() {
        violations.push("there is no pyproject.toml".to_owned());
    } else {
        for line in std::fs::read_to_string(&package)
            .unwrap_or_default()
            .lines()
        {
            // `dependencies = []` is the only shape that needs nothing
            // installed; a name in the list is something a reviewer would have
            // to fetch, and one of these fixtures is about a provider whose
            // client a real deployment would declare here.
            let line = line.trim();
            if line.starts_with("dependencies")
                && !line.replace(' ', "").ends_with("dependencies=[]")
            {
                violations.push(format!(
                    "pyproject.toml declares a dependency, so the fixture is not runnable without an install: `{line}`"
                ));
            }
        }
    }

    let entry_points = declared_entry_points(dir);
    if entry_points.is_empty() {
        violations.push("scenario.json declares no entry point to run".to_owned());
    }
    for (role, command) in entry_points {
        let Some(entry) = command.strip_prefix(PYTHON) else {
            violations.push(format!(
                "the {role} command must run `python` directly, and it says `{command}` \
                 (`python3` is a Microsoft Store alias where these fixtures run, and it runs nothing)"
            ));
            continue;
        };
        let entry = entry.split_whitespace().next().unwrap_or_default();
        if !dir.join(entry).is_file() {
            violations.push(format!(
                "the {role} command names {entry}, which is not a file in the fixture"
            ));
        }
    }

    for source in python_files(dir) {
        let text = std::fs::read_to_string(&source).unwrap_or_default();
        for module in python_imports(&text, dir) {
            violations.push(format!(
                "{} imports `{module}`, which is not in ALLOWED_PYTHON_MODULES and is not a file in the fixture",
                source
                    .strip_prefix(dir)
                    .unwrap_or(source.as_path())
                    .display()
            ));
        }
    }

    violations
}

/// The commands a fixture's `scenario.json` says a reviewer runs, in the order
/// the file spells them.
///
/// Read out of the same document the reviewers and the eval runner read, so the
/// command each language's checker holds to is the command those readers run.
fn declared_entry_points(dir: &Path) -> Vec<(String, String)> {
    let path = dir.join("scenario.json");
    if !path.is_file() {
        return Vec::new();
    }
    let document = read_json(&path);
    let Some(entries) = document.get("entry_points").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = entries
        .iter()
        .filter_map(|(role, command)| {
            command
                .as_str()
                .map(|command| (role.clone(), command.to_owned()))
        })
        .collect();
    found.sort();
    found
}

/// Every `.py` file under a directory, sorted, so a run is reproducible.
fn python_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "py") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The top-level modules a Python file imports that are neither the fixture's
/// own nor on [`ALLOWED_PYTHON_MODULES`].
///
/// The analogue of [`bare_requires`], with the one difference the language
/// imposes: Python has no `node:` marker, so a top-level name is either a
/// standard-library module, a module of the project, or something that would
/// have to be installed. The first is the list, the second is a file or package
/// in the fixture, and anything else is refused.
fn python_imports(text: &str, dir: &Path) -> Vec<String> {
    let local = local_python_modules(dir);
    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let modules = if let Some(rest) = line.strip_prefix("from ") {
            match rest.split_once(" import") {
                Some((module, _)) => module.trim(),
                None => continue,
            }
        } else if let Some(rest) = line.strip_prefix("import ") {
            rest.split('#').next().unwrap_or_default().trim()
        } else {
            continue;
        };
        for name in modules.split(',') {
            let name = name.split_whitespace().next().unwrap_or_default();
            let top = name.split('.').next().unwrap_or_default();
            if top.is_empty()
                || top.starts_with('.')
                || top == "__future__"
                || ALLOWED_PYTHON_MODULES.contains(&top)
                || local.iter().any(|module| module == top)
            {
                continue;
            }
            found.push(top.to_owned());
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Every module name the fixture itself provides: a `.py` file's stem, or a
/// directory holding an `__init__.py`.
fn local_python_modules(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        let entries: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        let is_package_dir = current != dir
            && entries
                .iter()
                .any(|path| path.file_name().and_then(|name| name.to_str()) == Some("__init__.py"));
        if is_package_dir && let Some(name) = current.file_name().and_then(|name| name.to_str()) {
            names.push(name.to_owned());
        }
        for path in entries {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "py")
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                names.push(stem.to_owned());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Every reason a Rust fixture is not runnable with nothing installed.
///
/// The Rust half of [`python_runnable_violations`], and the differences are the
/// language's rather than this file's. `cargo` is the one tool and it is already
/// required to build this repository, so there is no interpreter name to pin and
/// no module list to compare; what has to hold instead is that the manifest
/// declares nothing to fetch, that the project is a workspace of its own — or
/// `cargo` refuses to build a package sitting under another manifest's directory,
/// which is exactly where these fixtures sit — and that the reviewer's entry
/// point is a script. The script rather than `cargo test` in the fixture's own
/// directory, because `cargo` writes `Cargo.lock` and `target/` next to the
/// manifest it reads and the repository's `.gitignore` entry for `target/` is
/// anchored at the root: the script copies the project somewhere else first.
fn rust_runnable_violations(dir: &Path) -> Vec<String> {
    let mut violations = Vec::new();

    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
        violations.push("there is no Cargo.toml".to_owned());
        return violations;
    }
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();

    // A name under `[dependencies]` is something `cargo` fetches the first time a
    // check runs, and these fixtures run with no network.
    let mut under_dependencies = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            under_dependencies = line == "[dependencies]";
            continue;
        }
        if under_dependencies && !line.is_empty() && !line.starts_with('#') {
            violations.push(format!(
                "Cargo.toml declares a dependency, so the fixture is not runnable without a fetch: `{line}`"
            ));
        }
    }

    if !text.lines().any(|line| line.trim() == "[workspace]") {
        violations.push(
            "Cargo.toml does not declare a workspace of its own, so cargo would refuse to build a \
             package that sits under this checkout's manifest"
                .to_owned(),
        );
    }

    if !dir.join("scripts").join("check.ps1").is_file() {
        violations.push("there is no scripts/check.ps1 for a reviewer to run".to_owned());
    }

    let entry_points = declared_entry_points(dir);
    if entry_points.is_empty() {
        violations.push("scenario.json declares no entry point to run".to_owned());
    }
    for (role, command) in entry_points {
        if !command.starts_with(CARGO) {
            violations.push(format!(
                "the {role} command must run `cargo` directly, and it says `{command}`"
            ));
        }
    }

    violations
}

// --- the fixtures --------------------------------------------------------

#[test]
fn every_typescript_fixture_is_a_directory_this_repository_ships() {
    for id in TYPESCRIPT_FIXTURES {
        let dir = fixture_dir(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json"
        );
        assert!(
            dir.join("README.md").is_file(),
            "{id} has no README.md for a non-programmer to read"
        );
    }
}

#[test]
fn every_fixture_scenario_parses_and_satisfies_the_expectation_schema() {
    let schema = expectation_schema();
    for id in every_fixture_this_task_implemented() {
        let document = scenario(id);
        let violations = schema_violations(&document, &schema);
        assert!(
            violations.is_empty(),
            "{id}/scenario.json does not satisfy {}:\n  {}",
            schema["title"].as_str().unwrap_or("the schema"),
            violations.join("\n  ")
        );
    }
}

#[test]
fn no_fixture_this_task_implemented_is_still_marked_as_waiting_for_the_task_graph() {
    // The marker means "a stub a later task must replace". A fixture that ships
    // source, a start command and a README while still carrying it would be
    // graded by nothing and owned by nobody.
    for id in every_fixture_this_task_implemented() {
        let text = scenario_text(id);
        assert!(
            !text.contains(NOT_IMPLEMENTED),
            "fixtures/adversarial/{id}/scenario.json is still a stub"
        );
    }
}

#[test]
fn every_shipped_fixture_directory_corresponds_to_a_manifest_case() {
    // The other direction of the check below, and the one that catches a
    // fixture nobody grades: a directory with no manifest entry is a test whose
    // result nothing reads. The exemptions are named, each with its reason, in
    // `FIXTURES_WITHOUT_A_MANIFEST_CASE`, and
    // `every_fixture_without_a_manifest_case_is_one_named_here` checks them
    // against the manifest rather than taking them on trust.
    let manifest = manifest();
    let cases = manifest["cases"]
        .as_array()
        .expect("the manifest lists cases");
    for id in shipped_fixture_ids() {
        let graded = cases
            .iter()
            .any(|case| case["id"].as_str() == Some(id.as_str()));
        assert!(
            graded || manifest_exemption(&id).is_some(),
            "fixtures/adversarial/{id} has no case in evaluation/acceptance-manifest.json, \
             and is not named in FIXTURES_WITHOUT_A_MANIFEST_CASE either"
        );
    }
}

#[test]
fn every_fixture_without_a_manifest_case_is_one_named_here() {
    // Both directions, so the list can neither widen nor go stale. An entry for
    // an id the manifest *does* grade would be a graded fixture excused from
    // the severity agreement below, and an entry that is not a shipped fixture
    // would be a reason nothing reads.
    let shipped = shipped_fixture_ids();
    for (id, reason) in FIXTURES_WITHOUT_A_MANIFEST_CASE {
        assert!(
            !reason.trim().is_empty(),
            "{id} is excused from the manifest agreement with no reason given"
        );
        assert!(
            shipped.iter().any(|directory| directory == id),
            "{id} is excused from the manifest agreement and is not a fixture this repository ships"
        );
        assert!(
            manifest_case(id).is_none(),
            "{id} is excused from the manifest agreement, and the manifest has a case for it"
        );
    }
}

#[test]
fn every_case_this_task_implements_has_a_fixture_directory() {
    // The forward direction: a case named here with no directory is a case
    // nothing can run.
    let shipped = shipped_fixture_ids();
    for id in every_fixture_this_task_implemented() {
        assert!(
            shipped.iter().any(|shipped| shipped == id),
            "{id} is claimed by this task and has no fixture directory"
        );
    }
}

#[test]
fn every_fixture_agrees_with_the_manifest_about_id_severity_and_blocking() {
    for id in every_fixture_this_task_implemented() {
        let document = scenario(id);
        // The one exemption, and it is checked rather than assumed by the test
        // above. Everything else here — the id the document claims — is
        // asserted for every fixture whether the manifest grades it or not.
        assert_eq!(
            document["id"].as_str(),
            Some(id),
            "{id}/scenario.json names a different id"
        );
        if manifest_exemption(id).is_some() {
            continue;
        }
        let case = manifest_case(id).unwrap_or_else(|| panic!("{id} is not in the manifest"));

        assert_eq!(
            document["expected_severity"].as_str(),
            case["expected_severity"].as_str(),
            "{id}/scenario.json and the manifest disagree about severity"
        );
        assert_eq!(
            document["release_blocking"].as_bool(),
            case["release_blocking"].as_bool(),
            "{id}/scenario.json and the manifest disagree about release_blocking"
        );
    }
}

#[test]
fn every_fixture_forbids_the_false_green_shape() {
    for id in every_fixture_this_task_implemented() {
        let document = scenario(id);
        let blocking = document["release_blocking"].as_bool().unwrap_or(false);
        let violations = false_green_violations(&document, blocking);
        assert!(
            violations.is_empty(),
            "{id}/scenario.json does not forbid the false-green shape:\n  {}",
            violations.join("\n  ")
        );
    }
}

#[test]
fn every_runnable_fixture_directory_is_named_in_this_file() {
    // The other adversarial directories are scenario stubs that later tasks
    // (P14-T003 and the ones after them) fill in with their own language. A
    // directory that ships a `package.json` is a Node application and this file
    // owns those; one that ships a `pyproject.toml` or a `requirements.txt` is
    // a Python application, and PYTHON_FIXTURES is where those are named; one
    // that ships a `Cargo.toml` is a Rust application, and RUST_FIXTURES is
    // where those are named. If one appears that is not named in any list, add
    // it, so its entry point, its schema and its false-green rule are checked
    // here too.
    for id in shipped_fixture_ids() {
        let dir = fixture_dir(&id);
        let node = dir.join("package.json").is_file();
        let python = dir.join("pyproject.toml").is_file() || dir.join("requirements.txt").is_file();
        let rust = dir.join("Cargo.toml").is_file();
        if node {
            assert!(
                TYPESCRIPT_FIXTURES.contains(&id.as_str()),
                "fixtures/adversarial/{id} is a runnable Node fixture that TYPESCRIPT_FIXTURES does not name"
            );
        }
        if python {
            assert!(
                PYTHON_FIXTURES.contains(&id.as_str()),
                "fixtures/adversarial/{id} is a runnable Python fixture that PYTHON_FIXTURES does not name"
            );
        }
        if rust {
            assert!(
                RUST_FIXTURES.contains(&id.as_str()),
                "fixtures/adversarial/{id} is a runnable Rust fixture that RUST_FIXTURES does not name"
            );
        }
    }
}

#[test]
fn every_fixture_is_runnable_with_nothing_installed_but_node() {
    for id in TYPESCRIPT_FIXTURES {
        let violations = runnable_violations(&fixture_dir(id));
        assert!(
            violations.is_empty(),
            "{id} is not runnable as shipped:\n  {}",
            violations.join("\n  ")
        );
    }
}

#[test]
fn every_python_fixture_is_runnable_with_nothing_installed_but_python() {
    for id in PYTHON_FIXTURES {
        let violations = python_runnable_violations(&fixture_dir(id));
        assert!(
            violations.is_empty(),
            "{id} is not runnable as shipped:\n  {}",
            violations.join("\n  ")
        );
    }
}

#[test]
fn every_python_fixture_is_a_directory_this_repository_ships() {
    for id in PYTHON_FIXTURES {
        let dir = fixture_dir(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json"
        );
        assert!(
            dir.join("README.md").is_file(),
            "{id} has no README.md for a non-programmer to read"
        );
    }
    for id in TYPESCRIPT_FIXTURES {
        assert!(
            !PYTHON_FIXTURES.contains(id),
            "{id} is in both lists, so one of the two halves is misdescribed"
        );
    }
}

#[test]
fn every_rust_fixture_is_runnable_with_nothing_installed_but_cargo() {
    for id in RUST_FIXTURES {
        let violations = rust_runnable_violations(&fixture_dir(id));
        assert!(
            violations.is_empty(),
            "{id} is not runnable as shipped:\n  {}",
            violations.join("\n  ")
        );
    }
}

#[test]
fn every_claim_fixture_is_a_directory_this_repository_ships() {
    // The `P14-T004` fixtures, and the one thing they must not grow. A
    // recording has no project to run: a `package.json`, a `pyproject.toml` or a
    // `Cargo.toml` in one of these directories would put it into a runnability
    // sweep that runs the fixture, and there is nothing here to run — the
    // scenario declares a stream and the test that grades it is in `sure-core`.
    for id in CLAIM_FIXTURES {
        let dir = fixture_dir(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json, so there is no stream to grade"
        );
        assert!(
            dir.join("README.md").is_file(),
            "{id} has no README.md for a non-programmer to read"
        );
        for manifest in ["package.json", "pyproject.toml", "requirements.txt"] {
            assert!(
                !dir.join(manifest).is_file(),
                "{id} ships a {manifest}: these fixtures are recordings, and a runnability \
                 manifest would claim a project they do not have"
            );
        }
    }
    for id in CLAIM_FIXTURES {
        assert!(
            !TYPESCRIPT_FIXTURES.contains(id)
                && !PYTHON_FIXTURES.contains(id)
                && !RUST_FIXTURES.contains(id),
            "{id} is in a language list as well, so one of the two halves is misdescribed"
        );
    }
}

#[test]
fn every_rust_fixture_is_a_directory_this_repository_ships() {
    for id in RUST_FIXTURES {
        let dir = fixture_dir(id);
        assert!(dir.is_dir(), "{} is not a directory", dir.display());
        assert!(
            dir.join("scenario.json").is_file(),
            "{id} has no scenario.json"
        );
        assert!(
            dir.join("README.md").is_file(),
            "{id} has no README.md for a non-programmer to read"
        );
    }
    // The pair is the point of this task's fixture: one half is the other's
    // control, and the test that runs both is
    // `crates/sure-core/tests/rust_fixture_apps.rs`. A third Rust fixture would
    // need that test's control claim read again, which is why the count is held
    // here rather than assumed.
    assert_eq!(
        RUST_FIXTURES.len(),
        2,
        "the Rust fixtures are a fail/pass pair, and this file's claim is about exactly two"
    );
    for id in RUST_FIXTURES {
        assert!(
            !TYPESCRIPT_FIXTURES.contains(id) && !PYTHON_FIXTURES.contains(id),
            "{id} is in two lists, so one of the two halves is misdescribed"
        );
    }
}

#[test]
fn every_fixture_names_the_check_it_expects_to_fire() {
    for id in every_fixture_this_task_implemented() {
        let document = scenario(id);
        let outcomes = document["required_outcomes"]
            .as_array()
            .expect("required_outcomes is an array");
        assert!(
            !outcomes.is_empty(),
            "{id}/scenario.json requires no outcomes at all"
        );

        let names_a_module = outcomes.iter().any(|outcome| {
            outcome
                .get("module")
                .and_then(Value::as_str)
                .is_some_and(|module| module.starts_with("crates/sure-core/src/"))
        });
        assert!(
            names_a_module,
            "{id}/scenario.json names no check module, so nothing can grade it"
        );

        // And a named module is a file, not a path someone typed. A scenario
        // whose `module` was renamed out from under it would otherwise go on
        // reading as though a check with that name exists, and a reader
        // following it would find nothing.
        for outcome in outcomes {
            for key in ["module", "reading_module"] {
                let Some(named) = outcome.get(key).and_then(Value::as_str) else {
                    continue;
                };
                let path = root().join(named);
                assert!(
                    path.is_file(),
                    "{id}/scenario.json names {named} as its {key}, and there is no such file"
                );
            }
        }
    }
}

// --- the checkers themselves ---------------------------------------------

#[test]
fn the_schema_checker_rejects_a_document_that_breaks_the_schema() {
    let schema = expectation_schema();

    assert!(
        schema_violations(&serde_json::json!({}), &schema).len() >= 2,
        "an empty document must fail every required key"
    );
    assert!(
        schema_violations(
            &serde_json::json!({ "id": "x", "required_outcomes": [{ "kind": "finding" }] }),
            &schema
        )
        .is_empty(),
        "a conforming document must pass"
    );
    assert!(
        schema_violations(
            &serde_json::json!({ "id": 7, "required_outcomes": [{ "kind": "finding" }] }),
            &schema
        )
        .iter()
        .any(|violation| violation.contains("`id`")),
        "a numeric id must be refused"
    );
    assert!(
        schema_violations(
            &serde_json::json!({ "id": "x", "required_outcomes": ["finding"] }),
            &schema
        )
        .iter()
        .any(|violation| violation.contains("required_outcomes[0]")),
        "an array of strings must not pass for an array of objects"
    );
}

#[test]
fn the_false_green_checker_rejects_a_scenario_without_one() {
    assert!(
        !false_green_violations(&serde_json::json!({ "forbidden_outcomes": [] }), true).is_empty(),
        "an empty forbidden list must be refused"
    );
    assert!(
        !false_green_violations(
            &serde_json::json!({
                "forbidden_outcomes": [{ "kind": "silent", "description": "nothing is reported" }]
            }),
            true
        )
        .is_empty(),
        "a forbidden list that never names a false green must be refused"
    );
    assert!(
        !false_green_violations(
            &serde_json::json!({
                "forbidden_outcomes": [
                    { "kind": "false_green", "description": "reported as passing" }
                ]
            }),
            true
        )
        .is_empty(),
        "a release-blocking case with one forbidden outcome must be refused"
    );
    assert!(
        false_green_violations(
            &serde_json::json!({
                "forbidden_outcomes": [
                    { "kind": "false_green", "description": "reported as passing or verified" },
                    { "kind": "false_green", "description": "the project aggregates to green" }
                ]
            }),
            true
        )
        .is_empty(),
        "a case that forbids both shapes must pass"
    );
}

#[test]
fn the_runnable_checker_rejects_a_project_that_needs_an_install() {
    let scratch = root()
        .join("target")
        .join("tmp")
        .join("fixture-apps-checker");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("src")).unwrap();
    std::fs::write(scratch.join("src").join("main.js"), "require('express');\n").unwrap();

    // A package that needs an install, and a start script that names nothing.
    std::fs::write(
        scratch.join("package.json"),
        "{\n  \"dependencies\": { \"express\": \"4\" },\n  \"scripts\": {\n    \"start\": \"node src/missing.js\",\n    \"test\": \"npm run something\"\n  }\n}\n",
    )
    .unwrap();

    let violations = runnable_violations(&scratch);
    assert!(
        violations.iter().any(|v| v.contains("express")),
        "a package dependency must be refused: {violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.contains("missing.js")),
        "a start script naming a file that is not there must be refused: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("must run node directly")),
        "a script that is not a node command must be refused: {violations:?}"
    );

    // And the same project, fixed, passes — so the checker is not refusing
    // everything it is shown.
    std::fs::write(
        scratch.join("src").join("main.js"),
        "require('node:path');\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("package.json"),
        "{\n  \"scripts\": { \"start\": \"node src/main.js\", \"test\": \"node src/main.js\" }\n}\n",
    )
    .unwrap();
    assert!(
        runnable_violations(&scratch).is_empty(),
        "a project with no dependencies and a real entry point must pass"
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn the_require_reader_sees_local_and_builtin_modules_and_not_packages() {
    assert!(bare_requires("const fs = require('node:fs');").is_empty());
    assert!(bare_requires("const a = require('./a');").is_empty());
    assert!(bare_requires("const b = require(\"../b\");").is_empty());
    assert_eq!(
        bare_requires("const express = require('express');"),
        ["express".to_owned()]
    );
    assert_eq!(
        bare_requires("require('left-pad');\nrequire(\"lodash\");"),
        ["left-pad".to_owned(), "lodash".to_owned()]
    );
}

#[test]
fn the_python_import_reader_sees_local_and_stdlib_modules_and_not_packages() {
    let fixture = fixture_dir("missing-config");

    assert!(python_imports("import os\n", &fixture).is_empty());
    assert!(python_imports("from pathlib import Path\n", &fixture).is_empty());
    assert!(python_imports("from dataclasses import dataclass, field\n", &fixture).is_empty());
    assert!(python_imports("import os, sys\n", &fixture).is_empty());
    assert!(python_imports("from . import settings\n", &fixture).is_empty());
    // The fixture's own package, which is a directory holding an `__init__.py`
    // and not a name on the allowed list.
    assert!(python_imports("from app import settings\n", &fixture).is_empty());
    assert_eq!(
        python_imports("import requests\n", &fixture),
        ["requests".to_owned()]
    );
    assert_eq!(
        python_imports("from stripe import Charge\nimport yaml\n", &fixture),
        ["stripe".to_owned(), "yaml".to_owned()]
    );
}

#[test]
fn the_python_runnable_checker_rejects_a_project_that_is_not_stdlib_only() {
    let scratch = root()
        .join("target")
        .join("tmp")
        .join("fixture-apps-python-checker");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("app")).unwrap();
    std::fs::write(scratch.join("app").join("__init__.py"), "").unwrap();
    std::fs::write(scratch.join("app").join("main.py"), "import requests\n").unwrap();
    std::fs::write(
        scratch.join("scenario.json"),
        "{\n  \"id\": \"x\",\n  \"required_outcomes\": [],\n  \"entry_points\": {\n    \"start\": \"python3 app/missing.py\",\n    \"test\": \"python app/missing.py\"\n  }\n}\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("pyproject.toml"),
        "[project]\nname = \"x\"\ndependencies = [\"requests\"]\n",
    )
    .unwrap();

    let violations = python_runnable_violations(&scratch);
    assert!(
        violations.iter().any(|v| v.contains("requests")),
        "a third-party import must be refused: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("must run `python` directly")),
        "an entry point that runs python3 must be refused: {violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.contains("missing.py")),
        "an entry point naming a file that is not there must be refused: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("declares a dependency")),
        "a declared dependency must be refused: {violations:?}"
    );

    // And the same project, fixed, passes — so the checker is not refusing
    // everything it is shown.
    std::fs::write(scratch.join("app").join("main.py"), "import sqlite3\n").unwrap();
    std::fs::write(
        scratch.join("scenario.json"),
        "{\n  \"id\": \"x\",\n  \"required_outcomes\": [],\n  \"entry_points\": { \"start\": \"python app/main.py\" }\n}\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("pyproject.toml"),
        "[project]\nname = \"x\"\ndependencies = []\n",
    )
    .unwrap();
    assert!(
        python_runnable_violations(&scratch).is_empty(),
        "a stdlib-only project with a real entry point must pass: {:?}",
        python_runnable_violations(&scratch)
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn the_rust_runnable_checker_rejects_a_project_that_would_fetch_or_write() {
    let scratch = root()
        .join("target")
        .join("tmp")
        .join("fixture-apps-rust-checker");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("src")).unwrap();
    std::fs::write(scratch.join("src").join("main.rs"), "fn main() {}\n").unwrap();

    // A manifest with something to fetch, no workspace of its own, and an entry
    // point that is not cargo — and no script for a reviewer to run.
    std::fs::write(
        scratch.join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("scenario.json"),
        "{\n  \"id\": \"x\",\n  \"required_outcomes\": [],\n  \"entry_points\": { \"test\": \"python scripts/check.py\" }\n}\n",
    )
    .unwrap();

    let violations = rust_runnable_violations(&scratch);
    assert!(
        violations.iter().any(|v| v.contains("serde")),
        "a declared dependency must be refused: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("workspace of its own")),
        "a package that is not a workspace of its own must be refused: {violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.contains("check.ps1")),
        "a fixture with no script for a reviewer must be refused: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("must run `cargo` directly")),
        "an entry point that is not cargo must be refused: {violations:?}"
    );

    // And the same project, fixed, passes — so the checker is not refusing
    // everything it is shown.
    std::fs::create_dir_all(scratch.join("scripts")).unwrap();
    std::fs::write(scratch.join("scripts").join("check.ps1"), "# a check\n").unwrap();
    std::fs::write(
        scratch.join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("scenario.json"),
        "{\n  \"id\": \"x\",\n  \"required_outcomes\": [],\n  \"entry_points\": { \"test\": \"cargo test\" }\n}\n",
    )
    .unwrap();
    assert!(
        rust_runnable_violations(&scratch).is_empty(),
        "a dependency-free project with a script and a cargo entry point must pass: {:?}",
        rust_runnable_violations(&scratch)
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

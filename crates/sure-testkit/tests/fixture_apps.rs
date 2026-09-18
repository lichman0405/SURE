//! The adversarial fixture applications, checked as artefacts.
//!
//! `P14-T001`'s acceptance is one sentence:
//!
//! > *Fake payment/auth/email/dead button/route/demo cases runnable with
//! > expected outcomes.*
//!
//! Three words in that sentence are checkable without running SURE at all, and
//! this file checks them:
//!
//! - **runnable** — the fixture has a start command a reviewer with nothing but
//!   Node installed can run, it names a file that exists, and it needs no
//!   package installed first;
//! - **expected outcomes** — the fixture carries a `scenario.json` that parses
//!   and satisfies `schemas/fixture-expectation.schema.json`, and that agrees
//!   with `evaluation/acceptance-manifest.json` about how serious the case is;
//! - **the false-green rule** — a release-blocking fixture says, in machine
//!   readable form, that "passing" is an outcome SURE must not produce.
//!
//! What is deliberately **not** here: whether the defect is actually detected.
//! That claim needs SURE's scanners, which this crate cannot see, and it lives
//! in `crates/sure-core/tests/adversarial_fixture_detection.rs` where the
//! scanners are reachable.
//!
//! # Why the checkers are functions rather than assertions inlined in the tests
//!
//! `repository_shape.rs` states the rule this file follows: *a check like this is
//! only worth having if it can fail.* Every checker below is therefore a plain
//! function returning the reasons it rejected a value, and every one of them is
//! fed a hand-built case that it **must** reject. A checker that returned no
//! violations for everything would fail those cases, so it cannot pass by being
//! vacuous.

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

/// The marker the task graph leaves on a fixture that is still a stub.
const NOT_IMPLEMENTED: &str = "to_be_implemented_by_task_graph";

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
    for id in TYPESCRIPT_FIXTURES {
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
    for id in TYPESCRIPT_FIXTURES {
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
    // result nothing reads.
    let manifest = manifest();
    let cases = manifest["cases"]
        .as_array()
        .expect("the manifest lists cases");
    for id in shipped_fixture_ids() {
        assert!(
            cases
                .iter()
                .any(|case| case["id"].as_str() == Some(id.as_str())),
            "fixtures/adversarial/{id} has no case in evaluation/acceptance-manifest.json"
        );
    }
}

#[test]
fn every_case_this_task_implements_has_a_fixture_directory() {
    // The forward direction: a case named here with no directory is a case
    // nothing can run.
    let shipped = shipped_fixture_ids();
    for id in TYPESCRIPT_FIXTURES {
        assert!(
            shipped.iter().any(|shipped| shipped == id),
            "{id} is claimed by this task and has no fixture directory"
        );
    }
}

#[test]
fn every_fixture_agrees_with_the_manifest_about_id_severity_and_blocking() {
    for id in TYPESCRIPT_FIXTURES {
        let document = scenario(id);
        let case = manifest_case(id).unwrap_or_else(|| panic!("{id} is not in the manifest"));

        assert_eq!(
            document["id"].as_str(),
            Some(*id),
            "{id}/scenario.json names a different id"
        );
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
    for id in TYPESCRIPT_FIXTURES {
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
    // (P14-T002, P14-T003 and the ones after them) fill in with their own
    // language. A directory that ships a `package.json` is a Node application,
    // and this task owns those: if one appears that is not named above, add it
    // to TYPESCRIPT_FIXTURES so its entry point, its schema and its false-green
    // rule are checked here too.
    for id in shipped_fixture_ids() {
        if !fixture_dir(&id).join("package.json").is_file() {
            continue;
        }
        assert!(
            TYPESCRIPT_FIXTURES.contains(&id.as_str()),
            "fixtures/adversarial/{id} is a runnable Node fixture that TYPESCRIPT_FIXTURES does not name"
        );
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
fn every_fixture_names_the_check_it_expects_to_fire() {
    for id in TYPESCRIPT_FIXTURES {
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

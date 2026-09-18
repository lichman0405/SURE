//! Harness packages contain no duplicated check engine.
//!
//! P1-T001 acceptance: "Harness packages contain no duplicated check engine."
//!
//! ADR 0004 states the rule; these assertions are what makes it survive. Each
//! one is paired with a case that must fail, so a checker that always returned
//! "thin" would not pass.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sure_testkit::ThinnessFinding;
use sure_testkit::integrations::{self, IntegrationFile};

fn root() -> PathBuf {
    sure_testkit::repository_root().join("integrations")
}

fn loaded() -> Vec<IntegrationFile> {
    integrations::load(&root()).expect("integrations directory must be readable")
}

/// The core-owned user-facing wording, taken from the core rather than copied.
///
/// A copy here would drift from the constant it is supposed to guard, which is
/// the failure this whole check exists to catch.
fn frozen_sentences() -> Vec<&'static str> {
    vec![sure_domain::status::NO_TRUSTED_INTENT_LIMITATION]
}

#[test]
fn harness_packages_are_thin() {
    let files = loaded();
    assert!(
        !files.is_empty(),
        "no integration files were read, so this check proves nothing"
    );
    let mut findings = integrations::findings(&files, &frozen_sentences());
    findings.extend(integrations::manifest_reference_findings(&root(), &files));
    assert!(
        findings.is_empty(),
        "integration packages stopped being thin:\n{}",
        findings
            .iter()
            .map(|f| format!("  - {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_real_launchers_are_found_and_are_launchers() {
    // If the loader silently returned no launchers, the check above would pass
    // for the wrong reason.
    let files = loaded();
    let launchers: Vec<&IntegrationFile> = files.iter().filter(|f| f.is_launcher()).collect();
    assert!(
        launchers.len() >= 4,
        "expected the claude-code and cursor hook launchers, found {:?}",
        launchers.iter().map(|f| &f.relative).collect::<Vec<_>>()
    );
    for launcher in &launchers {
        assert!(
            integrations::reaches_core(&launcher.text),
            "{} does not reach the core, yet the tree is expected to be thin",
            launcher.relative
        );
    }
}

// --- the checker itself -------------------------------------------------

fn file(relative: &str, text: &str) -> IntegrationFile {
    IntegrationFile {
        relative: relative.to_owned(),
        path: PathBuf::from(relative),
        text: text.to_owned(),
    }
}

#[test]
fn a_launcher_that_never_reaches_the_core_is_reported() {
    let files = [file("h/scripts/decide.ps1", "Write-Output '{}'\n")];
    let findings = integrations::findings(&files, &frozen_sentences());
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, ThinnessFinding::LauncherDoesNotReachCore { file } if file.ends_with("decide.ps1"))),
        "a launcher that never calls the core must be reported: {findings:?}"
    );
}

#[test]
fn a_launcher_that_grew_into_a_program_is_reported() {
    let mut text = String::from("$bin = $env:SURE_BIN\n");
    while text.lines().count() <= integrations::MAX_LAUNCHER_LINES {
        text.push_str("$x = 1\n");
    }
    let files = [file("h/scripts/big.ps1", &text)];
    let findings = integrations::findings(&files, &frozen_sentences());
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, ThinnessFinding::LauncherTooLarge { .. })),
        "an oversized launcher must be reported: {findings:?}"
    );
}

#[test]
fn a_copied_frozen_sentence_is_reported() {
    let sentence = sure_domain::status::NO_TRUSTED_INTENT_LIMITATION;
    let files = [file(
        "h/scripts/talk.sh",
        &format!("SURE_BIN=sure\necho \"{sentence}\"\n"),
    )];
    let findings = integrations::findings(&files, &frozen_sentences());
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, ThinnessFinding::LauncherEmbedsFrozenWording { .. })),
        "a restated verdict sentence must be reported: {findings:?}"
    );
}

#[test]
fn a_check_engine_shipped_as_source_is_reported() {
    let files = [file("h/src/engine.rs", "fn check() {}\n")];
    let findings = integrations::findings(&files, &frozen_sentences());
    assert!(
        findings.iter().any(
            |f| matches!(f, ThinnessFinding::EngineArtefact { extension, .. } if extension == "rs")
        ),
        "Rust source inside an integration package must be reported: {findings:?}"
    );
}

#[test]
fn a_hook_manifest_pointing_at_a_missing_script_is_reported() {
    // A name that no other run can compute, for the reason `scan/mod.rs`'s
    // missing-directory fixture already records: `%TEMP%` is shared, and a
    // fixture a test did not create is one something else can take away.
    //
    // This test was one of five whose scratch directory carried a name every run
    // reused, and it was seen to fail with `Os { code: 3, kind: NotFound }` on the
    // write below — a write into a directory `create_dir_all` had just made.
    // **What takes the directory away is not known.** The obvious explanation, a
    // deletion by the previous run still finishing when this one starts, did not
    // survive a probe: 3000 rounds of exactly this shape — fixed name, create,
    // write, remove, repeat — failed zero times, and so did 3000 with a unique
    // name. What is established is narrower and is what this change acts on: all
    // five failures were on paths that every run shared, and a path no other
    // process can compute is a path no other process can remove. The five are
    // listed in `progress/HANDOFF.md`, with which of them were proven and which
    // were not.
    let dir = std::env::temp_dir().join(format!(
        "sure-testkit-thinness-missing-{}",
        std::process::id()
    ));
    let hooks = dir.join("hooks");
    std::fs::create_dir_all(&hooks).expect("temp dir");
    let manifest = hooks.join("hooks.json");
    std::fs::write(
        &manifest,
        r#"{"hooks":{"Stop":[{"command":"powershell -File .\\scripts\\gone.ps1 stop"}]}}"#,
    )
    .expect("write manifest");

    let files = [IntegrationFile {
        relative: "h/hooks/hooks.json".to_owned(),
        path: manifest,
        text: std::fs::read_to_string(hooks.join("hooks.json")).expect("read back"),
    }];
    let findings = integrations::manifest_reference_findings(&dir, &files);
    assert!(
        findings.iter().any(|f| matches!(
            f,
            ThinnessFinding::HookManifestReferencesMissingFile { referenced, .. }
            if referenced.contains("gone.ps1")
        )),
        "a hook wired to a missing script must be reported: {findings:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cursor_synthetic_fixtures_are_valid_json_with_expected_shape() {
    let fixtures = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("fixtures");
    assert!(
        fixtures.is_dir(),
        "cursor synthetic fixtures directory must exist"
    );

    let entries = std::fs::read_dir(&fixtures).expect("fixtures directory is readable");
    let mut count = 0;
    for entry in entries {
        let entry = entry.expect("fixture entry readable");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        count += 1;
        let text = std::fs::read_to_string(&path).expect("fixture file readable");
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("fixture must be valid JSON");
        let object = value.as_object().expect("fixture must be a JSON object");
        for key in ["event", "harness_session_id", "source"] {
            assert!(
                object.contains_key(key),
                "{} must contain '{key}'",
                path.display()
            );
        }
        assert_eq!(
            object.get("source").and_then(|v| v.as_str()),
            Some("cursor"),
            "{} must declare source as 'cursor'",
            path.display()
        );
    }
    assert!(
        count >= 6,
        "expected at least six cursor synthetic fixtures, found {count}"
    );
}

#[test]
fn claude_code_synthetic_fixtures_are_valid_json_with_expected_shape() {
    let fixtures = sure_testkit::repository_root()
        .join("integrations")
        .join("claude-code")
        .join("fixtures");
    assert!(
        fixtures.is_dir(),
        "claude-code synthetic fixtures directory must exist"
    );

    let entries = std::fs::read_dir(&fixtures).expect("fixtures directory is readable");
    let mut count = 0;
    for entry in entries {
        let entry = entry.expect("fixture entry readable");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        count += 1;
        let text = std::fs::read_to_string(&path).expect("fixture file readable");
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("fixture must be valid JSON");
        let object = value.as_object().expect("fixture must be a JSON object");
        for key in ["event", "harness_session_id", "source"] {
            assert!(
                object.contains_key(key),
                "{} must contain '{key}'",
                path.display()
            );
        }
        assert_eq!(
            object.get("source").and_then(|v| v.as_str()),
            Some("claude-code"),
            "{} must declare source as 'claude-code'",
            path.display()
        );
    }
    assert!(
        count >= 4,
        "expected at least four claude-code synthetic fixtures, found {count}"
    );
}

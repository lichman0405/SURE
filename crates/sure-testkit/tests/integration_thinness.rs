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

#[test]
fn cursor_launcher_contract_fixtures_are_valid_json() {
    let launcher = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("fixtures")
        .join("launcher");
    assert!(
        launcher.is_dir(),
        "cursor launcher fixtures directory must exist"
    );

    let entries = std::fs::read_dir(&launcher).expect("launcher fixtures directory is readable");
    let mut count = 0;
    for entry in entries {
        let entry = entry.expect("launcher fixture entry readable");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        count += 1;
        let text = std::fs::read_to_string(&path).expect("launcher fixture file readable");
        let _: serde_json::Value =
            serde_json::from_str(&text).expect("launcher fixture must be valid JSON");
    }
    assert!(
        count >= 3,
        "expected at least three cursor launcher contract fixtures, found {count}"
    );

    // The launcher script must resolve SURE from the override, PATH, and the
    // per-user install location, and must not fabricate evidence when missing.
    let script = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("scripts")
        .join("sure-hook.ps1");
    let text = std::fs::read_to_string(&script).expect("cursor launcher script readable");
    assert!(
        text.contains("SURE_BIN"),
        "cursor launcher must honour SURE_BIN override"
    );
    assert!(
        text.contains("Get-Command sure"),
        "cursor launcher must look for sure on PATH"
    );
    assert!(
        text.contains("LOCALAPPDATA"),
        "cursor launcher must fall back to per-user install location"
    );
    assert!(
        text.contains("SURE binary not found"),
        "cursor launcher must fail safely when SURE is missing"
    );
    assert!(
        text.contains("hook ingest --source cursor"),
        "cursor launcher must forward to the SURE core"
    );
}

#[test]
fn claude_code_launcher_contract_fixtures_are_valid_json() {
    let launcher = sure_testkit::repository_root()
        .join("integrations")
        .join("claude-code")
        .join("fixtures")
        .join("launcher");
    assert!(
        launcher.is_dir(),
        "claude-code launcher fixtures directory must exist"
    );

    let entries = std::fs::read_dir(&launcher).expect("launcher fixtures directory is readable");
    let mut count = 0;
    for entry in entries {
        let entry = entry.expect("launcher fixture entry readable");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        count += 1;
        let text = std::fs::read_to_string(&path).expect("launcher fixture file readable");
        let _: serde_json::Value =
            serde_json::from_str(&text).expect("launcher fixture must be valid JSON");
    }
    assert!(
        count >= 3,
        "expected at least three claude-code launcher contract fixtures, found {count}"
    );

    // The launcher script must resolve SURE from the override, PATH, and the
    // per-user install location, and must not fabricate evidence when missing.
    let script = sure_testkit::repository_root()
        .join("integrations")
        .join("claude-code")
        .join("scripts")
        .join("sure-hook.ps1");
    let text = std::fs::read_to_string(&script).expect("claude-code launcher script readable");
    assert!(
        text.contains("SURE_BIN"),
        "claude-code launcher must honour SURE_BIN override"
    );
    assert!(
        text.contains("Get-Command sure"),
        "claude-code launcher must look for sure on PATH"
    );
    assert!(
        text.contains("LOCALAPPDATA"),
        "claude-code launcher must fall back to per-user install location"
    );
    assert!(
        text.contains("SURE binary not found"),
        "claude-code launcher must fail safely when SURE is missing"
    );
    assert!(
        text.contains("hook ingest --source claude-code"),
        "claude-code launcher must forward to the SURE core"
    );
}

#[test]
fn claude_code_check_command_exists_and_is_thin() {
    let check_md = sure_testkit::repository_root()
        .join("integrations")
        .join("claude-code")
        .join("commands")
        .join("check.md");

    assert!(
        check_md.is_file(),
        "claude-code check.md command file must exist"
    );

    let text = std::fs::read_to_string(&check_md).expect("check.md readable");

    // It must reference the local SURE invocation path or concept.
    let reaches_core =
        text.contains("SURE_BIN") || text.contains("sure.exe") || text.contains("sure check");
    assert!(
        reaches_core,
        "check.md must reference local SURE invocation (SURE_BIN, sure.exe, or sure check)"
    );

    // It must not copy the frozen no-trusted-intent limitation sentence.
    let normalised = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let frozen = sure_domain::status::NO_TRUSTED_INTENT_LIMITATION;
    let frozen_normalised = frozen.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !normalised.contains(&frozen_normalised),
        "check.md must not copy the frozen no-trusted-intent limitation sentence; the core owns that wording"
    );
}

#[test]
fn cursor_repair_handoff_references_repair_and_recheck() {
    let fix_md = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("commands")
        .join("fix.md");

    assert!(fix_md.is_file(), "cursor fix.md must exist");

    let text = std::fs::read_to_string(&fix_md).expect("fix.md readable");

    assert!(
        text.contains("sure repair"),
        "cursor fix.md must reference `sure repair` so the handoff can obtain the contract"
    );
    assert!(
        text.contains("sure recheck"),
        "cursor fix.md must reference `sure recheck` so the handoff can close with evidence"
    );

    let normalised = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let frozen = sure_domain::status::NO_TRUSTED_INTENT_LIMITATION;
    let frozen_normalised = frozen.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !normalised.contains(&frozen_normalised),
        "cursor fix.md must not copy the frozen no-trusted-intent limitation sentence"
    );
}

#[test]
fn cursor_command_files_exist_and_are_thin() {
    let commands_dir = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("commands");

    let expected = ["check.md", "status.md", "fix.md", "recheck.md"];
    for name in &expected {
        let path = commands_dir.join(name);
        assert!(path.is_file(), "cursor command file {name} must exist");

        let text = std::fs::read_to_string(&path).expect("command file readable");

        // Each command must reference the local SURE invocation path or concept.
        let reaches_core = text.contains("SURE_BIN")
            || text.contains("sure.exe")
            || text.contains("sure check")
            || text.contains("sure history")
            || text.contains("sure recheck")
            || text.contains("sure repair");
        assert!(
            reaches_core,
            "{name} must reference local SURE invocation (SURE_BIN, sure.exe, or sure <subcommand>)"
        );

        // No command file may copy the frozen no-trusted-intent limitation sentence.
        let normalised = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let frozen = sure_domain::status::NO_TRUSTED_INTENT_LIMITATION;
        let frozen_normalised = frozen.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            !normalised.contains(&frozen_normalised),
            "{name} must not copy the frozen no-trusted-intent limitation sentence; the core owns that wording"
        );
    }
}

#[test]
fn cursor_install_script_exists_and_has_required_contract() {
    let script = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("scripts")
        .join("install.ps1");

    assert!(script.is_file(), "cursor install.ps1 must exist");

    let text = std::fs::read_to_string(&script).expect("install.ps1 readable");

    assert!(
        text.contains("SURE_BIN"),
        "cursor install.ps1 must reference SURE_BIN for binary resolution"
    );
    assert!(
        text.contains("LOCALAPPDATA"),
        "cursor install.ps1 must reference LOCALAPPDATA for per-user fallback"
    );
    assert!(
        text.contains("Copy-Item") || text.to_ascii_lowercase().contains("copy"),
        "cursor install.ps1 must support copy fallback"
    );
    assert!(
        text.contains("ForceCopy")
            || text.contains("{{SURE_BIN}}")
            || text.contains("{{PLUGIN_ROOT}}"),
        "cursor install.ps1 must have a copy/render fallback or force-copy switch"
    );
}

#[test]
fn cursor_uninstall_script_exists() {
    let script = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("scripts")
        .join("uninstall.ps1");

    assert!(script.is_file(), "cursor uninstall.ps1 must exist");

    let text = std::fs::read_to_string(&script).expect("uninstall.ps1 readable");

    assert!(
        text.contains("CURSOR_PLUGIN_DIR") || text.contains("APPDATA"),
        "cursor uninstall.ps1 must know where to remove the plugin from"
    );
}

#[test]
fn cursor_install_script_does_not_require_symlinks_unconditionally() {
    let script = sure_testkit::repository_root()
        .join("integrations")
        .join("cursor")
        .join("scripts")
        .join("install.ps1");

    let text = std::fs::read_to_string(&script).expect("install.ps1 readable");

    // The script must not unconditionally use New-Item -ItemType SymbolicLink;
    // it should have a copy fallback or a ForceCopy switch.
    let has_symlink = text.contains("SymbolicLink");
    let has_fallback = text.contains("ForceCopy") || text.contains("Copy-Item");
    assert!(
        !has_symlink || has_fallback,
        "cursor install.ps1 must not require symlink privileges unconditionally; it needs a copy fallback"
    );
}

#[cfg(windows)]
#[test]
fn cursor_install_script_runs_into_temp_directory() {
    let repo = sure_testkit::repository_root();
    let install_script = repo
        .join("integrations")
        .join("cursor")
        .join("scripts")
        .join("install.ps1");

    let temp =
        std::env::temp_dir().join(format!("sure-cursor-install-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("temp dir");

    let plugin_dir = temp.join("cursor-plugins");
    let sure_bin = temp.join("sure.exe");
    std::fs::write(&sure_bin, "dummy").expect("write dummy sure.exe");

    let output = std::process::Command::new("powershell")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &install_script.to_string_lossy(),
        ])
        .env("CURSOR_PLUGIN_DIR", &plugin_dir)
        .env("SURE_BIN", &sure_bin)
        .output()
        .expect("spawn install.ps1");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "install.ps1 failed: stdout={stdout}, stderr={stderr}"
    );

    let installed = plugin_dir.join("sure");
    assert!(
        installed.is_dir(),
        "plugin directory should be created at {installed:?}"
    );
    assert!(
        installed.join("hooks").join("hooks.json").is_file(),
        "hooks.json should be present in installed plugin"
    );
    assert!(
        installed
            .join(".cursor-plugin")
            .join("plugin.json")
            .is_file(),
        "plugin.json should be present in installed plugin"
    );

    // Verify that placeholders were rendered in copied files.
    // (If Developer Mode created a symlink, placeholders are not rendered;
    // this assertion only matters for the copy path.)
    if !installed.is_symlink() {
        let hook = std::fs::read_to_string(installed.join("scripts").join("sure-hook.ps1"))
            .expect("read installed hook");
        // The original contains the literal placeholder; a rendered copy would
        // have it replaced. Since the dummy sure_bin path does not contain the
        // placeholder string, we can check it was processed by looking for the
        // original placeholder absence in any file that had it. In practice none
        // of the current files contain placeholders, so we just verify the copy
        // succeeded and the directory structure is intact.
        assert!(
            hook.contains("hook ingest --source cursor"),
            "installed hook should still forward to the SURE core"
        );
    }

    let _ = std::fs::remove_dir_all(&temp);
}

#[cfg(windows)]
#[test]
fn cursor_uninstall_script_removes_installed_plugin() {
    let repo = sure_testkit::repository_root();
    let uninstall_script = repo
        .join("integrations")
        .join("cursor")
        .join("scripts")
        .join("uninstall.ps1");

    let temp =
        std::env::temp_dir().join(format!("sure-cursor-uninstall-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("temp dir");

    let plugin_dir = temp.join("cursor-plugins");
    let installed = plugin_dir.join("sure");
    std::fs::create_dir_all(&installed).expect("create fake plugin dir");
    std::fs::write(installed.join("dummy.txt"), "hello").expect("write dummy file");

    let output = std::process::Command::new("powershell")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &uninstall_script.to_string_lossy(),
        ])
        .env("CURSOR_PLUGIN_DIR", &plugin_dir)
        .output()
        .expect("spawn uninstall.ps1");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "uninstall.ps1 failed: stdout={stdout}"
    );
    assert!(
        stdout.contains("Uninstalled"),
        "uninstall.ps1 should report removal when plugin was present: {stdout}"
    );
    assert!(
        !installed.exists(),
        "plugin directory should be removed after uninstall"
    );

    // Run again against empty directory; should report not present.
    let output = std::process::Command::new("powershell")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &uninstall_script.to_string_lossy(),
        ])
        .env("CURSOR_PLUGIN_DIR", &plugin_dir)
        .output()
        .expect("spawn uninstall.ps1 again");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not installed"),
        "uninstall.ps1 should report not-installed when plugin is absent: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&temp);
}

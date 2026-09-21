//! Reading `sure.yaml` from a real filesystem.
//!
//! P1-T003 acceptance: "Invalid config is actionable."
//!
//! `Config::from_yaml` is unit-tested against text, which cannot show that the
//! bytes on disk reach it unchanged, or that the path a user typed survives
//! into the message. Every case here goes through `Config::load`, in a real
//! directory whose *name* contains a space and a non-ASCII character — the two
//! things a Windows path is most likely to be mishandled over, and the two a
//! unit test taking a `&str` never touches.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::config::{Config, ConfigSource, ErrorKind, PrivacyMode, ReportFormat};

/// A scratch project directory that removes itself.
///
/// Created under the workspace's own `target/` rather than the system temp
/// directory. It is already ignored by Git and already excluded from any
/// packaging step, and it sits on the same volume as the checkout, which is
/// where a path-handling bug would actually appear.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "{test}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        // The space and the non-ASCII characters are the point of the fixture,
        // not decoration: a path SURE mishandles has to be reachable in a test.
        let path = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("sure 配置")
            .join(unique);
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Write raw bytes, so a test can produce a file that is not valid UTF-8.
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path.join(name);
        std::fs::write(&path, bytes)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
        path
    }

    fn write_yaml(&self, text: &str) -> PathBuf {
        self.write(Config::FILE_NAME, text.as_bytes())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Failing to clean up must not turn a passing test into a failing one.
        // Windows keeps directory handles open longer than Unix does, so a
        // scanner or an editor may still hold one.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn settings_reach_the_model_from_a_real_file() {
    let scratch = Scratch::new("load");
    let written = scratch.write_yaml("privacy:\n  mode: fully_local\n\nreport:\n  format: json\n");

    let loaded = Config::load(scratch.path()).expect("this file is one SURE accepts");
    assert_eq!(loaded.config.privacy.mode, PrivacyMode::FullyLocal);
    assert_eq!(loaded.config.report.format, ReportFormat::Json);

    // The provenance matters as much as the settings: a report that says a
    // check was turned off has to be able to say who turned it off.
    match &loaded.source {
        ConfigSource::File(path) => assert_eq!(path, &written),
        other => panic!("expected the settings to come from a file, got {other:?}"),
    }
    assert!(loaded.source.is_file());
    assert_eq!(loaded.searched, written);
}

#[test]
fn a_byte_order_mark_is_accepted_from_a_real_file() {
    // Notepad and several other Windows editors write one by default. The unit
    // test proves the parser tolerates the character; this proves the character
    // is what the file actually contains after a round trip through the disk.
    let scratch = Scratch::new("bom");
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend_from_slice(b"report:\n  format: json\n");
    scratch.write(Config::FILE_NAME, &bytes);

    let loaded = Config::load(scratch.path()).expect("a byte-order mark is not a syntax error");
    assert_eq!(loaded.config.report.format, ReportFormat::Json);
}

#[test]
fn carriage_returns_from_a_windows_editor_are_accepted() {
    let scratch = Scratch::new("crlf");
    scratch.write_yaml("privacy:\r\n  mode: fully_local\r\nreport:\r\n  format: json\r\n");

    let loaded = Config::load(scratch.path()).expect("CRLF line endings are normal on Windows");
    assert_eq!(loaded.config.privacy.mode, PrivacyMode::FullyLocal);
    assert_eq!(loaded.config.report.format, ReportFormat::Json);
}

#[test]
fn a_file_that_is_not_utf8_is_refused_and_the_message_names_it() {
    let scratch = Scratch::new("not-utf8");
    // What a file saved from an older editor in the local codepage looks like.
    let written = scratch.write(Config::FILE_NAME, b"report:\n  format: json # \xe8\xe9\n");

    let error = Config::load(scratch.path()).expect_err("invalid UTF-8 must not be guessed at");
    assert_eq!(error.kind(), &ErrorKind::NotUtf8);
    assert_eq!(error.file(), Some(written.as_path()));

    let text = error.to_string();
    assert!(
        text.contains(&written.display().to_string()),
        "the message does not name the file:\n{text}"
    );
    assert!(text.contains("Save the file as UTF-8"), "{text}");
}

#[test]
fn a_file_named_sure_yml_is_reported_rather_than_ignored() {
    // The failure this prevents: the user believes their settings are in force,
    // and SURE is running with defaults.
    let scratch = Scratch::new("near-miss");
    let written = scratch.write("sure.yml", b"report:\n  format: json\n");

    let error = Config::load(scratch.path()).expect_err("the near-miss must not be passed over");
    match error.kind() {
        ErrorKind::WrongFileName { found, expected } => {
            assert_eq!(found, &written);
            assert_eq!(expected, &scratch.path().join(Config::FILE_NAME));
        }
        other => panic!("expected a wrong-file-name error, got {other:?}"),
    }
    assert_eq!(error.file(), Some(written.as_path()));

    let text = error.to_string();
    assert!(text.contains("sure.yml"), "{text}");
    assert!(text.contains("Rename it to sure.yaml"), "{text}");
}

#[test]
fn a_project_with_no_configuration_file_is_not_an_error() {
    let scratch = Scratch::new("absent");

    let loaded = Config::load(scratch.path()).expect("declaring nothing is allowed");
    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.source, ConfigSource::NoFile);
    assert!(!loaded.source.is_file());
    // `searched` is always populated, even when nothing was found, so a caller
    // can tell the user which path was looked at.
    assert_eq!(loaded.searched, scratch.path().join(Config::FILE_NAME));
}

#[test]
fn a_directory_named_sure_yaml_is_reported_rather_than_treated_as_absent() {
    // A path that exists but is not a readable file would otherwise fall
    // through to "no configuration file" — a false green, since SURE would run
    // with defaults while something does occupy that name.
    let scratch = Scratch::new("directory-in-the-way");
    let written = scratch.path().join(Config::FILE_NAME);
    std::fs::create_dir(&written).unwrap();

    let error = Config::load(scratch.path()).expect_err("an unreadable path is not an absent one");
    match error.kind() {
        ErrorKind::Unreadable { message } => {
            assert_eq!(
                message, "it is a directory, not a file",
                "the operating system calls this an access failure, which sends the \
                 user looking for a permission problem"
            );
        }
        other => panic!("expected an unreadable-file error, got {other:?}"),
    }
    assert_eq!(error.file(), Some(written.as_path()));
}

#[test]
fn an_invalid_setting_names_the_file_it_is_in() {
    let scratch = Scratch::new("invalid");
    let written = scratch.write_yaml("execution:\n  modee: inspect_only\n");

    let error = Config::load(scratch.path()).expect_err("an unknown setting must stop the run");
    match error.kind() {
        ErrorKind::UnknownSetting {
            name, suggestion, ..
        } => {
            assert_eq!(name, "execution.modee");
            assert_eq!(suggestion.as_deref(), Some("mode"));
        }
        other => panic!("expected an unknown setting, got {other:?}"),
    }
    assert_eq!(error.file(), Some(written.as_path()));

    let text = error.to_string();
    assert!(text.contains("Did you mean `mode`?"), "{text}");
    let named = written.display().to_string();
    assert_eq!(
        text.matches(&named).count(),
        1,
        "the file should be named exactly once:\n{text}"
    );
}

#[test]
fn a_credential_in_a_real_file_is_refused_and_its_value_never_appears() {
    let scratch = Scratch::new("credential");
    let written = scratch.write_yaml(
        "analysis:\n  provider: openai_compatible\n  model: small\n  \
         endpoint: https://alice:hunter2@api.example.com/v1\n",
    );

    let error = Config::load(scratch.path()).expect_err("a project file may not carry a secret");
    assert_eq!(
        error.kind(),
        &ErrorKind::Credential {
            setting: "analysis.endpoint".to_owned()
        }
    );
    assert_eq!(error.file(), Some(written.as_path()));

    let text = error.to_string();
    assert!(
        !text.contains("hunter2"),
        "the password was printed:\n{text}"
    );
    assert!(
        !text.contains("alice"),
        "the user name was printed:\n{text}"
    );
    assert!(
        text.contains("analysis.endpoint"),
        "the setting is not named, so the user cannot find it:\n{text}"
    );
}

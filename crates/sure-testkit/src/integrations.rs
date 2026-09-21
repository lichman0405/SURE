//! Assertions that the harness packages stay thin.
//!
//! ADR 0004 says an integration package contains manifests, hooks, commands and
//! skills, and no checking logic. That is easy to state and easy to erode: the
//! first time a hook needs "just a little" judgement, a decision leaves the core
//! and stops being testable, auditable or consistent with the CLI.
//!
//! These checks are deliberately blunt. They are not a proof that no logic was
//! written in a shell script — proving that requires reading the script. They
//! catch the failure modes that actually happen: a launcher that never reaches
//! the core, a launcher that has grown into a program, and a launcher that has
//! copied a frozen sentence so it can phrase a verdict itself.
//!
//! A file that fails one of these is a prompt to look, not a verdict on it.

use std::fmt;
use std::path::{Path, PathBuf};

/// Launcher file extensions an integration package may contain.
pub const LAUNCHER_EXTENSIONS: &[&str] = &[
    "ps1", "psm1", "cmd", "bat", "sh", "bash", "zsh", "fish", "js", "mjs", "cjs", "ts", "py",
];

/// Source or binary artefacts that would mean a second engine shipped here.
pub const FORBIDDEN_EXTENSIONS: &[&str] = &["rs", "dll", "so", "dylib", "a", "rlib", "exe"];

/// The longest a launcher may be before it is a program rather than a launcher.
///
/// The intended shape is: read stdin, find the core, hand the bytes over. The
/// existing Windows and POSIX hooks are 9 and 17 lines. Anything approaching
/// this bound has started to make decisions of its own.
pub const MAX_LAUNCHER_LINES: usize = 40;

/// A file found under an integration package.
#[derive(Debug, Clone)]
pub struct IntegrationFile {
    /// Path relative to the integrations root, with `/` separators.
    pub relative: String,
    /// Absolute path on disk.
    pub path: PathBuf,
    /// File contents.
    pub text: String,
}

impl IntegrationFile {
    /// Lowercased extension, or an empty string when there is none.
    #[must_use]
    pub fn extension(&self) -> String {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
    }

    /// Whether this file is a launcher script.
    #[must_use]
    pub fn is_launcher(&self) -> bool {
        LAUNCHER_EXTENSIONS.contains(&self.extension().as_str())
    }

    /// Number of lines, counting a final line without a trailing newline.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.text.lines().count()
    }
}

/// A way an integration package departed from being thin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThinnessFinding {
    /// A launcher never reaches the core, so it can only be doing its own work.
    LauncherDoesNotReachCore {
        /// Relative path of the file.
        file: String,
    },
    /// A launcher has grown past the size at which it is still a launcher.
    LauncherTooLarge {
        /// Relative path of the file.
        file: String,
        /// Lines found.
        lines: usize,
        /// The bound.
        limit: usize,
    },
    /// A launcher contains a frozen user-facing sentence.
    ///
    /// The core owns that wording. A copy in an integration is a second source
    /// of truth for what SURE tells a user, and it will not be updated when the
    /// constant is.
    LauncherEmbedsFrozenWording {
        /// Relative path of the file.
        file: String,
    },
    /// A source or binary artefact is present, which is a second engine.
    EngineArtefact {
        /// Relative path of the file.
        file: String,
        /// The extension found.
        extension: String,
    },
    /// A hook or MCP manifest names a script that is not there.
    ///
    /// This is the quiet one: the harness reports nothing, the hook or the
    /// server never runs, and the session simply has no evidence in it.
    ManifestReferencesMissingFile {
        /// Relative path of the manifest.
        manifest: String,
        /// The missing file, as named by the manifest.
        referenced: String,
    },
    /// A hook or MCP manifest is not valid JSON.
    UnreadableManifest {
        /// Relative path of the manifest.
        manifest: String,
        /// What went wrong.
        detail: String,
    },
}

impl fmt::Display for ThinnessFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LauncherDoesNotReachCore { file } => write!(
                f,
                "{file}: never invokes the SURE core, so it cannot be forwarding evidence to it"
            ),
            Self::LauncherTooLarge { file, lines, limit } => write!(
                f,
                "{file}: {lines} lines, over the {limit}-line launcher bound; harness packages hold \
                 manifests and launchers, not logic (ADR 0004)"
            ),
            Self::LauncherEmbedsFrozenWording { file } => write!(
                f,
                "{file}: contains the frozen after-the-fact sentence; the core owns that wording \
                 and an integration must not restate a verdict"
            ),
            Self::EngineArtefact { file, extension } => write!(
                f,
                "{file}: '.{extension}' in an integration package is a second check engine; the \
                 core is the only implementation (ADR 0001)"
            ),
            Self::ManifestReferencesMissingFile {
                manifest,
                referenced,
            } => write!(
                f,
                "{manifest}: references '{referenced}', which does not exist; what it names would \
                 silently never run and the session would carry no evidence"
            ),
            Self::UnreadableManifest { manifest, detail } => {
                write!(f, "{manifest}: not valid JSON: {detail}")
            }
        }
    }
}

/// Failures that stop the integration tree from being inspected.
#[derive(Debug)]
pub enum Error {
    /// A directory could not be read.
    Io {
        /// The directory.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The integrations root does not exist.
    ///
    /// Reported rather than treated as "no integrations, nothing to check",
    /// because an empty result would read as a pass.
    MissingRoot {
        /// The directory that was expected.
        path: PathBuf,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Self::MissingRoot { path } => write!(
                f,
                "no integrations found at {}; an absent tree is not a passing check",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::MissingRoot { .. } => None,
        }
    }
}

/// Read every file under `integrations_root`, recursively.
pub fn load(integrations_root: &Path) -> Result<Vec<IntegrationFile>, Error> {
    if !integrations_root.is_dir() {
        return Err(Error::MissingRoot {
            path: integrations_root.to_path_buf(),
        });
    }
    let mut files = Vec::new();
    collect(integrations_root, integrations_root, &mut files)?;
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<IntegrationFile>) -> Result<(), Error> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        out.push(IntegrationFile {
            relative,
            path,
            text,
        });
    }
    Ok(())
}

/// Whether a launcher invokes the core.
///
/// Matched on the ways a launcher may legitimately name it: the documented
/// override variable, the Windows binary name, or a bare `sure` command token
/// as used by `command -v sure` and `Get-Command sure`.
#[must_use]
pub fn reaches_core(text: &str) -> bool {
    text.contains("SURE_BIN") || text.contains("sure.exe") || text.contains("sure ")
}

/// Every way the integration tree departs from being thin.
///
/// `frozen_sentences` are the exact user-facing strings the core owns; passing
/// them in from the core keeps this check from drifting from the wording it
/// guards.
#[must_use]
pub fn findings(files: &[IntegrationFile], frozen_sentences: &[&str]) -> Vec<ThinnessFinding> {
    let mut found = Vec::new();
    for file in files {
        let extension = file.extension();
        if FORBIDDEN_EXTENSIONS.contains(&extension.as_str()) {
            found.push(ThinnessFinding::EngineArtefact {
                file: file.relative.clone(),
                extension,
            });
            continue;
        }
        if !file.is_launcher() {
            continue;
        }
        if !reaches_core(&file.text) {
            found.push(ThinnessFinding::LauncherDoesNotReachCore {
                file: file.relative.clone(),
            });
        }
        let lines = file.line_count();
        if lines > MAX_LAUNCHER_LINES {
            found.push(ThinnessFinding::LauncherTooLarge {
                file: file.relative.clone(),
                lines,
                limit: MAX_LAUNCHER_LINES,
            });
        }
        let normalised = file.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if frozen_sentences.iter().any(|sentence| {
            normalised.contains(&sentence.split_whitespace().collect::<Vec<_>>().join(" "))
        }) {
            found.push(ThinnessFinding::LauncherEmbedsFrozenWording {
                file: file.relative.clone(),
            });
        }
    }
    found
}

/// The manifests that name a file the harness will run.
///
/// A hook manifest names a launcher, and an MCP manifest names the command that
/// is the server — which is a script in the packages that need one to resolve
/// the binary. Both are the same defect when the file is not there: the
/// harness starts nothing and says nothing, and the session has no evidence in
/// it. `hooks.json` sits in a `hooks/` directory and names files relative to
/// the package; the MCP manifests sit at the package root.
pub const MANIFEST_NAMES: &[&str] = &["hooks.json", "mcp.json", ".mcp.json"];

/// Check that every file a hook or MCP manifest points at exists.
///
/// Only path-shaped strings are considered: manifests also carry matchers,
/// shell names and hook-type words, and treating those as missing files would
/// bury a real breakage in noise.
#[must_use]
pub fn manifest_reference_findings(
    integrations_root: &Path,
    files: &[IntegrationFile],
) -> Vec<ThinnessFinding> {
    let mut found = Vec::new();
    for file in files {
        let name = file.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !MANIFEST_NAMES.contains(&name) {
            continue;
        }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&file.text);
        let value = match parsed {
            Ok(value) => value,
            Err(error) => {
                found.push(ThinnessFinding::UnreadableManifest {
                    manifest: file.relative.clone(),
                    detail: error.to_string(),
                });
                continue;
            }
        };
        // A hook manifest is one directory below the package it names files in;
        // an MCP manifest is the package's own file. Both name their scripts
        // relative to the package, which is what `${CLAUDE_PLUGIN_ROOT}` and
        // `${PLUGIN_ROOT}` are stripped down to below.
        let package_dir = match name {
            "hooks.json" => file
                .path
                .parent()
                .and_then(Path::parent)
                .unwrap_or(integrations_root),
            _ => file.path.parent().unwrap_or(integrations_root),
        };
        for reference in script_references(&value) {
            let normalised = reference
                .replace("${CLAUDE_PLUGIN_ROOT}", "")
                .replace("${PLUGIN_ROOT}", "")
                .replace("${workspaceFolder}", "")
                .replace('\\', "/");
            let normalised = normalised.trim();
            // Strip a leading `./` or `/` so the result stays *relative*.
            // `Path::join` replaces the whole path when handed something with a
            // root, which would turn `.../cursor` + `/scripts/x.ps1` into
            // `C:\scripts\x.ps1` and report a breakage that is not there.
            let mut normalised = normalised;
            while let Some(stripped) = normalised
                .strip_prefix("./")
                .or_else(|| normalised.strip_prefix('/'))
            {
                normalised = stripped;
            }
            if normalised.is_empty() {
                continue;
            }
            if !package_dir.join(normalised).is_file() {
                found.push(ThinnessFinding::ManifestReferencesMissingFile {
                    manifest: file.relative.clone(),
                    referenced: reference,
                });
            }
        }
    }
    found
}

/// Strings in a hook manifest that look like paths to a script.
fn script_references(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    match value {
        serde_json::Value::String(text) => {
            for token in text.split_whitespace() {
                let trimmed = token.trim_matches(|c: char| c == '"' || c == '\'');
                if LAUNCHER_EXTENSIONS
                    .iter()
                    .any(|ext| trimmed.to_ascii_lowercase().ends_with(&format!(".{ext}")))
                {
                    out.push(trimmed.to_owned());
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                out.extend(script_references(item));
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                out.extend(script_references(item));
            }
        }
        _ => {}
    }
    out
}

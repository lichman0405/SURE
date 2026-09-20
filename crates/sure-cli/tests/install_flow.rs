//! The Windows per-user install and uninstall flow, as a person meets it.
//!
//! `P15-T003`'s acceptance is that SURE installs the CLI to a documented
//! per-user path with no hidden background service and no administrator
//! requirement, that uninstall removes or preserves user data only with a clear
//! choice, and that the `PATH`/integration behavior is documented **and tested**.
//! All three are claims about what a process does to a filesystem, so this file
//! runs the two scripts and reads the filesystem back.
//!
//! # What it is really guarding
//!
//! `scripts/Install-Sure.ps1` installs into `%LOCALAPPDATA%\SURE\bin`, which is
//! where seven launcher scripts across five integration packages already resolve
//! `sure.exe` — and which is **also** where `crates/sure-core/src/paths/mod.rs`
//! puts the user's evidence, `sure.db`. So an uninstall that walks the install
//! directory would destroy the history SURE exists to keep. The tests below are
//! written so that the failure they exist to catch is not "a file was left
//! behind" but "a file that was not the install's was removed": every case
//! plants something of the user's first and checks it byte for byte afterwards.
//!
//! Nothing here touches the real `%LOCALAPPDATA%\SURE`. The flow is always given
//! an `-InstallRoot` under this repository's git-ignored `target/tmp`, and the
//! launchers are given `LOCALAPPDATA` pointing at the same scratch directory, so
//! what they resolve is the install this test made. That is the only place the
//! store of whoever runs the suite could be edited from, and it is not reached.
//!
//! # The archive these tests install
//!
//! `P15-T002`'s archive is produced by `scripts/Build-Release.ps1`, which refuses
//! to package without the release gate and takes minutes to run a release build.
//! A test cannot depend on that, so this file stages the *documented layout* —
//! one top-level directory named `sure-<version>-x86_64-pc-windows-msvc` holding
//! `sure.exe`, `LICENSE` and `RELEASE.txt`, with a `sha256sum`-format
//! `<archive>.sha256` beside it — around the binary cargo just built. What the
//! installer reads is a real ZIP with real bytes in it; what it is not is the
//! release build, and this comment is where that difference is stated rather
//! than left for a reader to assume.

// The whole flow is Windows-only: a per-user `%LOCALAPPDATA%` install, a
// PowerShell script, and launchers that resolve `%LOCALAPPDATA%\SURE\bin`.
// `crates/sure-testkit/tests/hook_failure_semantics.rs` covers the Unix half of
// the launchers, and `CLAUDE.md`'s portability requirement is about the Rust
// core, not about this file.
#![cfg(windows)]
// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::json;
use sure_cli::mcp::PROTOCOL_VERSION;
use sure_core::paths::STORE_FILE;

/// The binary this package builds, as cargo hands it to its integration tests.
///
/// It is what goes into the archive, so the bytes that reach `bin\sure.exe` and
/// the bytes the launcher starts are the ones cargo produced — not a stand-in
/// that would let a broken copy step pass.
const SURE: &str = env!("CARGO_BIN_EXE_sure");

// --- Where everything lives ---------------------------------------------

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}

fn install_script() -> PathBuf {
    repository_root().join("scripts").join("Install-Sure.ps1")
}

fn uninstall_script() -> PathBuf {
    repository_root().join("scripts").join("Uninstall-Sure.ps1")
}

/// The launcher `integrations/claude-code/.mcp.json` names as its MCP server.
///
/// It is the one launcher whose third resolution step carries
/// `[Environment]::GetFolderPath('LocalApplicationData')` as a fallback — the
/// tree's own precedent for not hand-rolling the known-folder lookup, which is
/// what both scripts' `Get-PerUserDataRoot` follows — and the one a test can
/// drive end to end, because a handshake and a tool list run no command and open
/// no store, while a hook ingest would write one.
fn claude_code_mcp_launcher() -> PathBuf {
    repository_root()
        .join("integrations")
        .join("claude-code")
        .join("scripts")
        .join("sure-mcp.ps1")
}

/// A directory of this test's own, under the workspace's git-ignored `target/tmp`.
///
/// Unique per call, and made unique by `create_dir` rather than by the name, so
/// that two tests in one process cannot be handed the same one. Never cleared
/// afterwards: a failed test's directory is the evidence of what it did, and
/// `target/tmp` is where this repository already keeps that.
///
/// The name carries a space, a non-ASCII character and a full stop, because
/// `CLAUDE.md` says to test paths with spaces and Unicode and this flow carries
/// one path through PowerShell's argument parsing, a ZIP, a JSON manifest and
/// back out. The install *root* below it still ends in `SURE`, because that leaf
/// is what the launcher scripts resolve and what `sure-core`'s `APP_DIR` names.
fn a_directory_of_our_own(what: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = repository_root()
        .join("target")
        .join("tmp")
        .join("sure install flow é中文 and spaces");
    std::fs::create_dir_all(&base)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
    for _ in 0..1_000 {
        let candidate = base.join(format!("{what}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("cannot create {}: {error}", candidate.display()),
        }
    }
    panic!("no free directory under {}", base.display());
}

// --- The PowerShell hosts ------------------------------------------------

/// The PowerShell that ships with Windows, found the way Windows finds it.
///
/// The same choice `crates/sure-cli/tests/mcp_protocol.rs::powershell()` makes,
/// for the same reason: it is the name a Windows machine can count on being
/// there, and a test should not depend on `PATH` any more than the scripts do.
fn windows_powershell() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    PathBuf::from(root).join("System32\\WindowsPowerShell\\v1.0\\powershell.exe")
}

/// PowerShell 7, when this machine has it.
fn powershell_seven() -> Option<PathBuf> {
    let from_path = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|directory| directory.join("pwsh.exe"))
            .find(|candidate| candidate.is_file())
    });
    from_path.or_else(|| {
        let installed = PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe");
        installed.is_file().then_some(installed)
    })
}

/// The hosts this suite drives the flow through.
///
/// The first is always Windows PowerShell, so every test that takes one host is
/// deterministic. PowerShell 7 is added when it is installed, because
/// `CLAUDE.md` names PowerShell 7+ for this repository's scripts and a flow that
/// was only ever driven by the older host would be a claim about the newer one
/// that nothing measured. `a_release_archive_installs_and_uninstalls_on_every_
/// host` is the test that uses all of them.
fn hosts() -> Vec<(&'static str, PathBuf)> {
    let mut found = vec![("Windows PowerShell", windows_powershell())];
    if let Some(seven) = powershell_seven() {
        found.push(("PowerShell 7", seven));
    }
    found
}

/// The host the single-host tests use.
fn a_host() -> (&'static str, PathBuf) {
    hosts().remove(0)
}

// --- Running things ------------------------------------------------------

/// What one script run did.
struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    fn of(output: Output) -> Self {
        Self {
            status: output.status.code().expect("the process exited on its own"),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Both streams, for a message that has to show why something failed.
    fn everything(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Run one of this repository's scripts with typed arguments.
///
/// A `Command` with an argument vector and no shell string, which is
/// `CLAUDE.md`'s Windows rule and is also why a path with a space or a non-ASCII
/// character survives: the quoting is `Command`'s, and `-File` hands the tokens
/// to the script as they were given.
fn run_script(host: &Path, script: &Path, arguments: &[&str]) -> Run {
    assert!(
        script.is_file(),
        "{} is not there, so there is nothing to test",
        script.display()
    );
    let output = Command::new(host)
        .arg("-NoProfile")
        .arg("-File")
        .arg(script)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("could not start {}: {error}", host.display()));
    Run::of(output)
}

// --- The archive the installer is given ----------------------------------

/// The staging script, written into a scratch directory and run by a host.
///
/// A file rather than `-Command`, because a command string built by formatting
/// paths into it is the shell-string construction `CLAUDE.md` rules out — and
/// these paths deliberately hold a space and a non-ASCII character.
const STAGE_AND_PACK: &str = r#"
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $PayloadExe,
    [Parameter(Mandatory)][string] $ScratchRoot,
    [Parameter(Mandatory)][string] $Version,
    [string] $Target = 'x86_64-pc-windows-msvc'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue
$name = "sure-$Version-$Target"
$stage = Join-Path (Join-Path $ScratchRoot 'stage') $name
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item -LiteralPath $PayloadExe -Destination (Join-Path $stage 'sure.exe') -Force
$license = "test fixture: the archive's LICENSE entry`n"
[System.IO.File]::WriteAllText((Join-Path $stage 'LICENSE'), $license, [System.Text.ASCIIEncoding]::new())
$release = "SURE $Version - a test fixture in the documented archive layout`n"
[System.IO.File]::WriteAllText((Join-Path $stage 'RELEASE.txt'), $release, [System.Text.ASCIIEncoding]::new())
$archive = Join-Path $ScratchRoot "$name.zip"
if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
[System.IO.Compression.ZipFile]::CreateFromDirectory($stage, $archive, [System.IO.Compression.CompressionLevel]::Optimal, $true)
# .NET rather than `Get-FileHash`, so that this fixture runs in the same
# environment the scripts are tested in: a PowerShell 7 session's `PSModulePath`
# reaching Windows PowerShell 5.1 leaves `Get-FileHash` undefined there while
# every other command either script uses still resolves. The scripts' own
# headers carry the measurement, and `Get-Sha256` in `Install-Sure.ps1` carries
# the diagnostic output it came from. No test here asserts that property: it is
# a fact about this machine's environment, not about the product, and a test
# asserting it would fail anywhere the parent session is not PowerShell 7.
$stream = [System.IO.File]::OpenRead($archive)
try { $hasher = [System.Security.Cryptography.SHA256]::Create(); try { $hash = $hasher.ComputeHash($stream) } finally { $hasher.Dispose() } } finally { $stream.Dispose() }
$digest = ([System.BitConverter]::ToString($hash) -replace '-', '').ToLowerInvariant()
$line = "$digest  $name.zip`n"
[System.IO.File]::WriteAllText("$archive.sha256", $line, [System.Text.Encoding]::ASCII)
Write-Output $archive
"#;

/// An archive in the layout `docs/development/RELEASE_PROCESS.md` records, with
/// its checksum, both inside `scratch`.
fn a_release_archive(host: &Path, scratch: &Path) -> (PathBuf, PathBuf) {
    let stager = scratch.join("stage-and-pack.ps1");
    // `run_script` hands this path to PowerShell and PowerShell starts it, so it
    // is a program and is put at its path by the door for programs: written
    // beside its name, closed there, renamed onto it, and never at a path with a
    // write descriptor still open. See `sure_testkit::program`.
    sure_testkit::write_program(&stager, STAGE_AND_PACK.as_bytes(), 0o755)
        .expect("the staging script can be written");
    let scratch_text = scratch.to_string_lossy().into_owned();
    let run = run_script(
        host,
        &stager,
        &[
            "-PayloadExe",
            SURE,
            "-ScratchRoot",
            &scratch_text,
            "-Version",
            "0.0.0-install-flow-test",
        ],
    );
    assert_eq!(
        run.status,
        0,
        "staging the test archive failed:\n{}",
        run.everything()
    );
    let archive = PathBuf::from(run.stdout.trim());
    let checksum = PathBuf::from(format!("{}.sha256", archive.display()));
    (archive, checksum)
}

/// Install `archive` into `root`, and say what the run did.
fn install(host: &Path, archive: &Path, root: &Path, extra: &[&str]) -> Run {
    let archive = archive.to_string_lossy().into_owned();
    let root = root.to_string_lossy().into_owned();
    let mut arguments = vec!["-Archive", archive.as_str(), "-InstallRoot", root.as_str()];
    arguments.extend_from_slice(extra);
    run_script(host, &install_script(), &arguments)
}

/// Uninstall from `root`, and say what the run did.
fn uninstall(host: &Path, root: &Path, extra: &[&str]) -> Run {
    let root = root.to_string_lossy().into_owned();
    let mut arguments = vec!["-InstallRoot", root.as_str()];
    arguments.extend_from_slice(extra);
    run_script(host, &uninstall_script(), &arguments)
}

// --- Criterion 1 and 3: it installs, and the launchers find it ------------

#[test]
fn an_archive_installs_where_the_launchers_already_look_for_it() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("install");
    let (archive, checksum) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");

    assert!(checksum.is_file(), "the staging script wrote no checksum");
    let run = install(&host, &archive, &root, &[]);
    assert_eq!(run.status, 0, "the install failed:\n{}", run.everything());

    // The path is the one seven launcher scripts resolve, and it is not a choice
    // made here: `integrations/claude-code/scripts/sure-mcp.ps1:16` and the six
    // beside it join this same name, and `grep -rn LOCALAPPDATA integrations/`
    // is the search that says so. Asserting the exact location is what keeps a
    // later change from quietly installing somewhere nothing looks.
    let installed = root.join("bin").join("sure.exe");
    assert!(
        installed.is_file(),
        "nothing was installed at {}",
        installed.display()
    );

    // The installed bytes are the bytes cargo built — and therefore the bytes
    // that went into the archive. A copy step that renamed the file but wrote
    // something else would pass every existence check above.
    assert_eq!(
        std::fs::read(&installed).expect("the installed binary can be read"),
        std::fs::read(SURE).expect("the built binary can be read"),
        "the installed binary is not the one that went into the archive"
    );

    // The manifest is what makes removal exact, so its being written, and its
    // naming a path relative to the install root, are part of the install rather
    // than an implementation detail. `bin\\sure.exe` is the manifest's own
    // spelling: a manifest recording an absolute path from the machine that
    // built it would not describe the machine that installed it.
    let manifest = std::fs::read_to_string(root.join("install-manifest.json"))
        .expect("the install writes a manifest");
    assert!(
        manifest.contains("sure-install-manifest") && manifest.contains(r"bin\\sure.exe"),
        "the manifest does not record what was installed:\n{manifest}"
    );
}

#[test]
fn the_claude_code_mcp_launcher_starts_the_installed_binary_with_no_path_entry() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("launcher");
    let (archive, _) = a_release_archive(&host, &scratch);

    // `LOCALAPPDATA` *is* the scratch directory, so the launcher's third
    // resolution step — `%LOCALAPPDATA%\SURE\bin\sure.exe` — names the install
    // this test just made. That is also the whole reason the suite never has to
    // touch the real one.
    let root = scratch.join("SURE");
    let installed = install(&host, &archive, &root, &[]);
    assert_eq!(installed.status, 0, "{}", installed.everything());

    // A `PATH` with nothing on it, so the launcher's second resolution step —
    // `Get-Command sure` — cannot answer either. What is left is only the third.
    let nothing_on_path = scratch.join("a path with nothing on it");
    std::fs::create_dir_all(&nothing_on_path).expect("the empty directory");

    let launcher = claude_code_mcp_launcher();
    assert!(launcher.is_file(), "{} is missing", launcher.display());
    let stream = format!(
        "{}\n{}\n",
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "install_flow.rs", "version": "0"},
            },
        }),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    );

    // `SURE_BIN` removed, `PATH` holding no `sure`, `LOCALAPPDATA` pointing at
    // the scratch root: the override is gone, the second step cannot answer, and
    // the only thing the third step can find is what the installer wrote.
    let output = run_launcher(&host, &launcher, &scratch, &nothing_on_path, &stream);
    let answers = format!("{}{}", output.stdout, output.stderr);
    assert_eq!(
        output.status, 0,
        "the launcher did not reach the installed binary:\n{answers}"
    );
    assert!(
        output.stdout.contains("\"id\":1") && output.stdout.contains("\"id\":2"),
        "the session did not answer both requests, so nothing proves the server started:\n{answers}"
    );
    assert!(
        !answers.contains("no SURE binary was found"),
        "the launcher fell through to its missing-binary branch:\n{answers}"
    );

    // And the falsifier for the sentence above: remove what the installer wrote
    // and the same run must fail. Without this, a launcher that ignored the
    // third step and somehow found another `sure` would pass the test above.
    std::fs::remove_file(root.join("bin").join("sure.exe")).expect("the installed binary");
    let without = run_launcher(&host, &launcher, &scratch, &nothing_on_path, &stream);
    assert_ne!(
        without.status, 0,
        "the launcher still started after the installed binary was removed, so the test above \
         was not measuring the install:\n{}{}",
        without.stdout, without.stderr
    );
    assert!(
        without.stderr.contains("no SURE binary was found")
            || without
                .stderr
                .contains("%LOCALAPPDATA%\\SURE\\bin\\sure.exe"),
        "the launcher failed for some other reason:\n{}",
        without.stderr
    );
}

/// Drive a launcher with a stream on its standard input.
///
/// The environment is the whole point of the test that calls this: `SURE_BIN`
/// removed, `PATH` set to a directory with nothing in it, and `LOCALAPPDATA`
/// pointing at the scratch root the install went into. Nothing else in the
/// launcher's three-step resolution can answer, so what it starts is what the
/// installer wrote.
fn run_launcher(
    host: &Path,
    launcher: &Path,
    localdata: &Path,
    nothing_on_path: &Path,
    stream: &str,
) -> Run {
    let mut child = Command::new(host)
        .arg("-NoProfile")
        .arg("-File")
        .arg(launcher)
        .env_remove("SURE_BIN")
        .env("LOCALAPPDATA", localdata)
        .env("PATH", nothing_on_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("could not start {}: {error}", launcher.display()));
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stream.as_bytes())
        .expect("the caller can write");
    child
        .wait_with_output()
        .map(Run::of)
        .expect("the launcher exits")
}

// --- Criterion 2: the experiment -----------------------------------------

/// The experiment the acceptance turns on.
///
/// The directory the install goes into is the directory `sure.db` lives in, so a
/// file named exactly what the user's evidence is named is planted beside the
/// install and checked byte for byte afterwards. If a future change makes the
/// uninstaller walk the directory, this is the test that reddens — and it
/// reddens on the bytes rather than on the exit status.
#[test]
fn uninstall_leaves_every_file_the_install_did_not_write() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("leaves-the-store");
    let (archive, _) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");

    let installed = install(&host, &archive, &root, &[]);
    assert_eq!(installed.status, 0, "{}", installed.everything());

    // Three files the install did not write: the user's evidence under the name
    // `crates/sure-core/src/paths/mod.rs` gives it, SQLite's write-ahead log for
    // it, and a file whose name has nothing to do with SURE at all.
    let evidence = b"three accepted checks, one finding, and a verdict";
    std::fs::write(root.join(STORE_FILE), evidence).expect("the planted store");
    std::fs::write(root.join(format!("{STORE_FILE}-wal")), b"wal").expect("the planted wal");
    std::fs::write(root.join("notes the user wrote.txt"), b"not SURE's").expect("the note");

    let run = uninstall(&host, &root, &[]);
    assert_eq!(run.status, 0, "the uninstall failed:\n{}", run.everything());

    assert_eq!(
        std::fs::read(root.join(STORE_FILE)).expect("the store is still there"),
        evidence,
        "the user's evidence was changed or removed by the uninstall"
    );
    assert!(
        root.join(format!("{STORE_FILE}-wal")).is_file(),
        "the write-ahead log beside the store was removed without being asked for"
    );
    assert!(
        root.join("notes the user wrote.txt").is_file(),
        "a file with no relation to SURE was removed"
    );
    assert!(
        !root.join("install-manifest.json").is_file(),
        "the install's own manifest is still there after a clean uninstall"
    );
    assert!(
        !root.join("bin").exists(),
        "the directory the install created is still there, and it was empty"
    );
    assert!(
        run.stdout.contains("Your history is still there"),
        "the run did not say the history was kept:\n{}",
        run.everything()
    );
}

/// The other half of "with clear choice": the switch removes the store, and only
/// the store.
#[test]
fn remove_user_data_deletes_the_store_by_name_and_leaves_everything_else() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("remove-user-data");
    let (archive, _) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");

    assert_eq!(install(&host, &archive, &root, &[]).status, 0);
    std::fs::write(root.join(STORE_FILE), b"history").expect("the planted store");
    std::fs::write(root.join(format!("{STORE_FILE}-wal")), b"wal").expect("the planted wal");
    std::fs::write(root.join("something else.db"), b"another database").expect("the other file");

    let run = uninstall(&host, &root, &["-RemoveUserData"]);
    assert_eq!(run.status, 0, "{}", run.everything());

    assert!(
        !root.join(STORE_FILE).exists(),
        "-RemoveUserData did not remove the store:\n{}",
        run.everything()
    );
    assert!(
        !root.join(format!("{STORE_FILE}-wal")).exists(),
        "-RemoveUserData did not remove the store's write-ahead log"
    );
    assert_eq!(
        std::fs::read(root.join("something else.db")).expect("the unrelated database survives"),
        b"another database",
        "-RemoveUserData removed a file that is not the store"
    );
}

/// A file at a recorded path whose bytes are no longer the ones the install
/// wrote is not the install's file any more, and is left.
#[test]
fn uninstall_keeps_a_binary_whose_bytes_changed_after_the_install() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("changed-bytes");
    let (archive, _) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");

    assert_eq!(install(&host, &archive, &root, &[]).status, 0);
    let installed = root.join("bin").join("sure.exe");
    let replacement = b"a different program entirely";
    std::fs::write(&installed, replacement).expect("replace the binary");

    let run = uninstall(&host, &root, &[]);
    assert_eq!(run.status, 0, "{}", run.everything());
    assert_eq!(
        std::fs::read(&installed).expect("the changed file is still there"),
        replacement,
        "the uninstall removed a file whose contents differ from what it wrote"
    );
    assert!(
        run.stdout.contains("not the ones this install wrote"),
        "the run did not say why it kept the file:\n{}",
        run.everything()
    );
    // The manifest is the only record of what is still there, so a run that left
    // something behind must leave the manifest too — otherwise a second attempt
    // could not know what the first one had found.
    assert!(
        root.join("install-manifest.json").is_file(),
        "the manifest was removed while a file it records is still on disk"
    );
}

/// The case the brief says is the real one: a directory holding files the
/// installer never wrote, and no manifest to say what it did write.
#[test]
fn uninstall_without_a_manifest_removes_nothing_and_says_so() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("no-manifest");
    let root = scratch.join("SURE");
    std::fs::create_dir_all(root.join("bin")).expect("the directory tree");
    std::fs::write(root.join(STORE_FILE), b"history from a development build")
        .expect("the planted store");
    std::fs::write(
        root.join("bin").join("sure.exe"),
        b"a binary from somewhere else",
    )
    .expect("the planted binary");

    let run = uninstall(&host, &root, &[]);
    assert_eq!(
        run.status,
        2,
        "an uninstall that cannot establish what was installed must refuse rather than \
         guess, and say so with a non-zero status:\n{}",
        run.everything()
    );
    assert_eq!(
        std::fs::read(root.join(STORE_FILE)).expect("the store survives"),
        b"history from a development build"
    );
    assert_eq!(
        std::fs::read(root.join("bin").join("sure.exe")).expect("the binary survives"),
        b"a binary from somewhere else"
    );
    assert!(
        run.stdout.contains("Nothing was removed"),
        "the run did not say that nothing was removed:\n{}",
        run.everything()
    );
}

/// `-WhatIf` prints the removals and makes none, and a dry run does not claim to
/// have uninstalled anything.
#[test]
fn what_if_removes_nothing_and_says_it_removed_nothing() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("what-if");
    let (archive, _) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");

    assert_eq!(install(&host, &archive, &root, &[]).status, 0);
    std::fs::write(root.join(STORE_FILE), b"history").expect("the planted store");

    let run = uninstall(&host, &root, &["-WhatIf"]);
    assert_eq!(run.status, 0, "{}", run.everything());
    assert!(
        root.join("bin").join("sure.exe").is_file(),
        "-WhatIf removed the installed binary"
    );
    assert!(root.join(STORE_FILE).is_file(), "-WhatIf removed the store");
    assert!(
        root.join("install-manifest.json").is_file(),
        "-WhatIf removed the manifest"
    );
    // The false green this guards: a dry run whose last line says "SURE is
    // uninstalled".
    assert!(
        !run.stdout.contains("SURE is uninstalled."),
        "a dry run claimed the program had been uninstalled:\n{}",
        run.everything()
    );
    assert!(
        run.stdout
            .contains("Nothing was removed and nothing was changed"),
        "the dry run did not say what it had done:\n{}",
        run.everything()
    );
}

/// Install must not write over a file it cannot account for either — the same
/// rule as uninstall, on the other side of the flow.
#[test]
fn install_refuses_to_replace_a_file_it_did_not_write_unless_forced() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("obstacle");
    let (archive, _) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");
    std::fs::create_dir_all(root.join("bin")).expect("the directory tree");
    let beside = b"a hand-placed program";
    std::fs::write(root.join("bin").join("sure.exe"), beside).expect("the planted binary");

    let refused = install(&host, &archive, &root, &[]);
    assert_eq!(
        refused.status,
        2,
        "the install replaced a file it did not write:\n{}",
        refused.everything()
    );
    assert_eq!(
        std::fs::read(root.join("bin").join("sure.exe")).expect("the planted binary survives"),
        beside
    );

    // `-Force` is the clear choice, and it is a choice rather than the default.
    let forced = install(&host, &archive, &root, &["-Force"]);
    assert_eq!(forced.status, 0, "{}", forced.everything());
    assert_ne!(
        std::fs::read(root.join("bin").join("sure.exe")).expect("the replaced binary"),
        beside,
        "-Force did not replace the file"
    );
}

/// An archive that does not match its checksum is not installed.
#[test]
fn an_archive_that_does_not_match_its_checksum_is_not_installed() {
    let (_, host) = a_host();
    let scratch = a_directory_of_our_own("bad-checksum");
    let (archive, _) = a_release_archive(&host, &scratch);
    let root = scratch.join("SURE");

    // One byte appended, and the `.sha256` left as it was. This is the mutation
    // `README`-level readers of a checksum file most often make: they check the
    // file is *there*.
    let mut bytes = std::fs::read(&archive).expect("the archive can be read");
    bytes.push(0);
    std::fs::write(&archive, &bytes).expect("the archive can be changed");

    let run = install(&host, &archive, &root, &[]);
    assert_eq!(
        run.status,
        2,
        "an archive that does not match its checksum was installed:\n{}",
        run.everything()
    );
    assert!(
        !root.join("bin").join("sure.exe").exists(),
        "something was installed from an archive that failed its checksum"
    );
    assert!(
        run.stdout.contains("does not match its checksum"),
        "the run did not say what was wrong:\n{}",
        run.everything()
    );
}

/// Criterion 1 says "without hidden background service or administrator
/// requirement", and this is that claim read out of the shipped code rather than
/// trusted. A paragraph saying "no service" is not evidence; the absence of the
/// call is.
#[test]
fn the_install_flow_never_reaches_for_a_service_an_administrator_or_the_machine() {
    let install_text = script_text(&install_script());
    let uninstall_text = script_text(&uninstall_script());

    let forbidden = [
        "New-Service",
        "Register-ScheduledTask",
        "New-ScheduledTask",
        "HKLM:",
        "HKEY_LOCAL_MACHINE",
        "Set-ItemProperty",
        "New-ItemProperty",
        "RunAs",
        "'Machine'",
        "\"Machine\"",
    ];
    for (which, text) in [
        ("Install-Sure.ps1", install_text.as_str()),
        ("Uninstall-Sure.ps1", uninstall_text.as_str()),
    ] {
        let code = code_lines(text);
        for pattern in forbidden {
            assert!(
                !code.iter().any(|line| line.contains(pattern)),
                "{which} reaches for {pattern}, which criterion 1 rules out"
            );
        }
    }

    // Neither script writes an environment variable, machine or user. The
    // installer *prints* the one line a person could run to put SURE on their own
    // `PATH`, and printing it is the point — this flow changes nothing and says
    // how to change it if you want it. So the rule is not "the call never
    // appears"; it is that every line carrying it is a line that only writes text.
    for (which, text) in [
        ("Install-Sure.ps1", install_text.as_str()),
        ("Uninstall-Sure.ps1", uninstall_text.as_str()),
    ] {
        for line in code_lines(text) {
            if !line.contains("SetEnvironmentVariable") {
                continue;
            }
            assert!(
                line.contains("Write-Host"),
                "{which} reaches SetEnvironmentVariable on a line that does more than print it, \
                 so it changes the environment: {line}"
            );
        }
    }

    // The uninstaller has no recursive delete at all, which is the structural
    // reason it cannot remove a directory that holds the user's evidence. The
    // installer has exactly two, and both name a directory it made under the
    // temporary directory — never the install root, and never a path a caller
    // typed.
    let uninstall_code = code_lines(&uninstall_text);
    for line in &uninstall_code {
        assert!(
            !line.contains("-Recurse"),
            "Uninstall-Sure.ps1 recurses, which is how a directory holding the user's store \
             gets removed: {line}"
        );
    }
    for line in code_lines(&install_text) {
        if !line.contains("-Recurse") {
            continue;
        }
        assert!(
            line.contains("$scratchRoot") || line.contains("$extractDirectory"),
            "Install-Sure.ps1 recurses into something that is not its own scratch directory: \
             {line}"
        );
    }

    // Every removal the uninstaller makes goes through one shape, so there is one
    // place to read rather than several: `Remove-Item -LiteralPath <one path>`.
    for line in &uninstall_code {
        if !line.contains("Remove-Item") {
            continue;
        }
        assert!(
            line.contains("-LiteralPath") && line.contains("-Force"),
            "Uninstall-Sure.ps1 removes something with a shape this test does not know: {line}"
        );
        assert!(
            !line.contains('*'),
            "Uninstall-Sure.ps1 removes by pattern rather than by name: {line}"
        );
    }
}

/// The store's name is the core's, not a second copy that can drift.
///
/// `-RemoveUserData` has to name the file it removes, and the name has to be the
/// one `crates/sure-core/src/paths/mod.rs` gives the store. A PowerShell file
/// cannot import a Rust constant, so this reads the list out of the script and
/// compares it with the constant: a rename on either side reddens here rather
/// than at somebody's data directory.
#[test]
fn the_uninstaller_names_the_store_the_core_actually_writes() {
    let text = script_text(&uninstall_script());
    let declaration = text
        .lines()
        .find(|line| line.contains("$DataFileNames = @("))
        .expect("the uninstaller declares the names it is allowed to remove");
    let names: Vec<String> = declaration
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    assert_eq!(
        names,
        [
            STORE_FILE.to_owned(),
            format!("{STORE_FILE}-wal"),
            format!("{STORE_FILE}-shm"),
        ],
        "the names the uninstaller removes are not the store and its two WAL sidecars"
    );
}

/// The two scripts resolve the per-user directory the same way, and the same way
/// the launchers do.
///
/// The functions are deliberately not shared through a third file: either script
/// has to keep working when someone has copied it somewhere on its own. What
/// keeps them from drifting is this test — the line that decides where the
/// per-user directory is, read out of both files and compared.
#[test]
fn the_two_scripts_resolve_the_per_user_directory_identically() {
    let mut blocks = Vec::new();
    for which in [install_script(), uninstall_script()] {
        let text = script_text(&which);
        let block = [
            "    $root = $env:LOCALAPPDATA",
            "    if (-not $root) { $root = [Environment]::GetFolderPath('LocalApplicationData') }",
        ];
        assert!(
            text.contains(&block.join("\n")),
            "{} does not resolve the per-user directory through %LOCALAPPDATA% and then the \
             platform's known folder, which is what `integrations/claude-code/scripts/\
             sure-mcp.ps1:16` does and what a hand-rolled read of the variable alone gets \
             wrong: it produces a *relative* path whenever the variable is unset",
            which.display()
        );
        blocks.push(block.join("\n"));
    }
    assert_eq!(
        blocks[0], blocks[1],
        "the two scripts decide where the per-user directory is by different means"
    );
}

/// One witness for the whole flow on every PowerShell it can find.
///
/// The single-host tests above take Windows PowerShell, which is always
/// installed. `CLAUDE.md` names PowerShell 7+ for this repository's scripts, and
/// a claim about that host that nothing ran is exactly what this repository
/// calls an unmeasured claim — so this runs the same three steps under each host
/// it can find and asserts the same outcome.
#[test]
fn the_install_flow_behaves_the_same_on_every_powershell_host() {
    for (label, host) in hosts() {
        let scratch = a_directory_of_our_own("host");
        let (archive, _) = a_release_archive(&host, &scratch);
        let root = scratch.join("SURE");

        let installed = install(&host, &archive, &root, &[]);
        assert_eq!(
            installed.status,
            0,
            "{label}: the install failed:\n{}",
            installed.everything()
        );
        assert!(
            root.join("bin").join("sure.exe").is_file(),
            "{label}: nothing was installed"
        );

        let evidence = b"history";
        std::fs::write(root.join(STORE_FILE), evidence).expect("the planted store");
        let removed = uninstall(&host, &root, &[]);
        assert_eq!(
            removed.status,
            0,
            "{label}: the uninstall failed:\n{}",
            removed.everything()
        );
        assert_eq!(
            std::fs::read(root.join(STORE_FILE)).expect("the store survives"),
            evidence,
            "{label}: the uninstall removed the store"
        );
        assert!(
            !root.join("install-manifest.json").is_file(),
            "{label}: the manifest survived a clean uninstall"
        );
    }
}

/// One of `scripts/*.ps1`, with its line endings normalized to LF.
///
/// `.gitattributes` declares `*.ps1 text eol=crlf`, so the working copy of these
/// files is CRLF after a fresh checkout and LF in the index and on this
/// machine's disk right now. Every comparison below is written against LF, and
/// normalizing here is what keeps a test from passing here and failing on the
/// next clone — which is the worst way for a test to be wrong.
fn script_text(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// The shipped code of a script, with comment lines removed.
///
/// The first `#` on a line starts a comment, which is true of the two files this
/// is used on because neither puts a `#` inside a string literal — the comments
/// in them say things like "no service, no scheduled task", and a scan that read
/// comment text as code would redden on the sentence that promises the
/// opposite.
fn code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim_end())
        .collect()
}

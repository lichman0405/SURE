//! What a hook launcher does when SURE cannot answer.
//!
//! P13-T007 acceptance: *"Fail-open/fail-closed behavior is explicit for each
//! harness/event and surfaced in docs."*
//!
//! # Why this file runs the launchers instead of reading them
//!
//! `integration_thinness.rs` already asserts what the launcher *text* contains —
//! that it names `SURE_BIN`, looks on `PATH`, and reaches the core. Those
//! assertions pass whatever the script then does with the status SURE returned,
//! which is the half a harness actually experiences. A launcher that resolved
//! the binary correctly and then dropped the status would satisfy every string
//! check in this repository and would leave a `block` unenforced on a real
//! session.
//!
//! So these tests spawn the packaged launcher, give it a stand-in for `sure`
//! that answers a status of the test's choosing, and assert the status and the
//! two streams that come back out. A change to a launcher's failure behaviour —
//! exit 0 where it used to relay 5, a decision invented where there was none,
//! silence where there was a note — fails here.
//!
//! # What is measured, and what is not
//!
//! Everything asserted here is SURE's own side of the contract: what the hook
//! process emits. **Nothing here observes a harness.** No Claude Code, Cursor,
//! Codex or Copilot process is started, so no test in this file is evidence
//! about what any of them does with a status SURE returned. That half is
//! recorded, per harness and per event, in
//! `docs/integrations/HOOK_FAILURE_SEMANTICS.md`, with the source for each cell
//! — and the last test in this file checks that the table covers every harness
//! and every event the packages actually wire.
//!
//! # Platform
//!
//! The PowerShell launchers run on Windows and the shell launchers on Unix, so
//! the launcher list is per-platform: four packages on Windows
//! (`claude-code`, `cursor`, `codex`, `copilot`) and three on Unix — `copilot`
//! ships a `sure-hook.ps1` and no `.sh` sibling, which its `README.md` accounts
//! for by calling the whole package a template. The assertions below are not
//! gated, so the same four tests run on both platforms over whichever launchers
//! exist there; the per-platform list is derived from the tree rather than
//! written down, and the table test separately requires a row for all four
//! packages' events on either platform.
//!
//! **The `.sh` launchers are run through `/bin/sh` rather than by path.** Git
//! records them `100644`, so a fresh checkout has no executable bit and the
//! manifests, which name the script directly, cannot start one there either.
//! The scripts use no bash-only syntax; running them under `sh` measures their
//! failure logic and not the checkout's file mode. The mode is recorded in
//! `docs/integrations/HOOK_FAILURE_SEMANTICS.md` as an open packaging gap.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code because a
// panic is a message nobody chose. In a test the panic *is* the report.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Where the per-harness failure table lives, for the test that keeps it true.
const TABLE: &str = "docs/integrations/HOOK_FAILURE_SEMANTICS.md";

/// The columns the per-event table is required to have, in order.
///
/// Seven, not four, because the two halves of the question must not be blended:
/// what SURE answers is measurable here and carries a test name, and what the
/// harness does with it is not measurable here at all and has to carry a source
/// or say that this repository cannot confirm it.
const COLUMNS: [&str; 7] = [
    "Harness",
    "Event",
    "What SURE answers normally",
    "When SURE cannot answer",
    "Evidence in this repository",
    "Failure semantics",
    "The harness side, and its source",
];

/// The three values the semantics column may hold.
///
/// `cannot confirm` is a value rather than an omission on purpose: the
/// acceptance is that the behaviour is *explicit* for each harness and event,
/// and "this repository has not established which way this one falls" is an
/// explicit statement. Leaving the cell blank is not.
const SEMANTICS: [&str; 3] = ["fail-open", "fail-closed", "cannot confirm"];

// --- the launchers on this platform -------------------------------------

/// Every hook launcher this platform can run, as `(harness, path)`.
///
/// Derived from the packages that name one, not from a list kept here: the
/// per-event table is checked against the same tree, so a package that gained a
/// launcher without gaining a row fails that test.
#[cfg(windows)]
fn hook_launchers() -> Vec<(String, PathBuf)> {
    hook_launchers_named("sure-hook.ps1")
}

#[cfg(unix)]
fn hook_launchers() -> Vec<(String, PathBuf)> {
    hook_launchers_named("sure-hook.sh")
}

/// The launch of one launcher, in whatever interpreter this platform has.
#[cfg(windows)]
fn launcher_command(script: &Path, event_kind: Option<&str>) -> Command {
    let mut command = Command::new(powershell());
    command.arg("-NoProfile").arg("-File").arg(script);
    if let Some(kind) = event_kind {
        command.arg(kind);
    }
    command
}

#[cfg(unix)]
fn launcher_command(script: &Path, event_kind: Option<&str>) -> Command {
    let mut command = Command::new("/bin/sh");
    command.arg(script);
    if let Some(kind) = event_kind {
        command.arg(kind);
    }
    command
}

/// A stand-in for `sure` that answers `code`, and answers 5 if it is given no
/// event on standard input.
///
/// A stand-in rather than a real build because these tests are about what the
/// launcher does with the status and the streams of whatever it resolved;
/// building a binary that is wrong on purpose would test the compiler. The
/// stdin check is what makes the forward half observable: a launcher that
/// resolved the binary and then dropped the payload would make a real SURE
/// answer 5, and this stand-in answers 5 too, so the test notices.
#[cfg(windows)]
fn stand_in(dir: &Path, code: u8) -> PathBuf {
    let path = dir.join(format!("sure-stand-in-{code}.cmd"));
    // `findstr` rather than `set /p`: measured 2026-09-19, `set /p line=` sets
    // nothing when the line arrives over the pipe PowerShell creates for a
    // native command, even though the bytes do arrive — `more` into a file
    // captured them, with the UTF-8 mark PowerShell 5.1 puts in front of them.
    // A stand-in that read nothing would answer 5 whatever code it was built
    // for, and the relay tests below would pass while proving nothing.
    let body = format!(
        "@echo off\r\n\
         findstr /r \".\" >nul 2>nul\r\n\
         if errorlevel 1 exit /b 5\r\n\
         echo stub-out {code}\r\n\
         echo stub-err {code} 1>&2\r\n\
         exit /b {code}\r\n"
    );
    std::fs::write(&path, body).expect("write the stand-in");
    path
}

/// The same, as a shell script.
#[cfg(unix)]
fn stand_in(dir: &Path, code: u8) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(format!("sure-stand-in-{code}.sh"));
    let body = format!(
        "#!/bin/sh\n\
         payload=$(cat)\n\
         [ -n \"$payload\" ] || exit 5\n\
         echo \"stub-out {code}\"\n\
         echo \"stub-err {code}\" >&2\n\
         exit {code}\n"
    );
    std::fs::write(&path, body).expect("write the stand-in");
    // The launcher only accepts `SURE_BIN` when the file is executable, so the
    // test's own stand-in has to carry the bit. That is a permission on a file
    // this test just made in its own scratch directory; nothing here needs a
    // privilege, a symlink or an administrator.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("mark the stand-in executable");
    path
}

/// The PowerShell to run a launcher with.
///
/// Named by absolute path when it can be, because one of these tests empties
/// `PATH` on purpose — that is how it makes `sure` unfindable — and a test that
/// could not find its own interpreter would fail for the wrong reason.
#[cfg(windows)]
fn powershell() -> PathBuf {
    if let Some(root) = std::env::var_os("SystemRoot") {
        let candidate = PathBuf::from(root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("powershell")
}

/// A scratch directory of this test's own.
///
/// Fixed per test and cleared first, like `integration_thinness.rs`'s
/// `launcher_scratch`, because a launcher test needs an *empty* `PATH` and
/// `LOCALAPPDATA` as much as it needs a stand-in in them.
fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sure-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

/// Run one launcher, with `bin` as SURE or with SURE nowhere to be found.
fn run_launcher(
    scratch: &Path,
    script: &Path,
    bin: Option<&Path>,
    event_kind: Option<&str>,
    payload: &str,
) -> Output {
    let mut command = launcher_command(script, event_kind);
    match bin {
        Some(bin) => {
            command.env("SURE_BIN", bin);
        }
        None => {
            // Where SURE could be, made unreachable in every way these launchers
            // look — and they look in four: `SURE_BIN` (removed), `PATH` (see
            // below), `$HOME/.cargo/bin/sure` (`HOME` points into an empty
            // directory, so that one cannot resolve), and the absolute
            // `/opt/homebrew/bin/sure` and `/usr/local/bin/sure` fallbacks that
            // no environment variable can move and that
            // `assert_sure_cannot_be_found` checks instead of assuming.
            // `LOCALAPPDATA` is the Windows third place; each is harmless on the
            // other platform, so this is one case rather than two.
            let nowhere = scratch.join("nowhere");
            std::fs::create_dir_all(&nowhere).expect("the empty directory");
            command
                .env_remove("SURE_BIN")
                .env("LOCALAPPDATA", &nowhere)
                .env("HOME", &nowhere);
            // Windows: the `.ps1` launchers call nothing outside the shell, so
            // an empty directory is the honest `PATH` there — nothing is on it.
            #[cfg(windows)]
            command.env("PATH", &nowhere);
            // Unix: the system utility directories instead, because the `.sh`
            // launchers are not in that position. Two of the three drain standard
            // input with `cat` (`cursor/scripts/sure-hook.sh:13`,
            // `codex/scripts/sure-hook.sh:17`), so an empty `PATH` hides `cat`
            // from them as well as hiding `sure`, and the shell writes its own
            // `cat: command not found` to stderr. Measured 2026-09-19 under Git
            // Bash: that is what the empty-`PATH` version produced on a launcher
            // whose missing-binary branch writes nothing at all, and it fails the
            // one assertion that says so. The launcher is right; the test's
            // `PATH` was wrong.
            #[cfg(unix)]
            {
                assert_sure_cannot_be_found(UNIX_UTILITY_PATH);
                command.env("PATH", UNIX_UTILITY_PATH);
            }
        }
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn the launcher");
    child
        .stdin
        .take()
        .expect("the pipe")
        .write_all(payload.as_bytes())
        .expect("write the event");
    child.wait_with_output().expect("the launcher exits")
}

/// The `PATH` a `.sh` launcher is given when SURE must not be findable.
///
/// The system utility directories, which is where `cat` lives on Linux and on
/// macOS — a POSIX shell script may reasonably use any of them, and hiding
/// `sure` must not hide them too. What makes this "SURE is nowhere to be
/// found" is `assert_sure_cannot_be_found` below rather than a hope.
#[cfg(unix)]
const UNIX_UTILITY_PATH: &str = "/usr/bin:/bin";

/// Fails unless no `sure` can be resolved through the `PATH` or the fallbacks.
///
/// The missing-binary case is only *missing* if the child cannot find a `sure`
/// anywhere the launcher would look. Two of the four lookups are controlled by
/// the test — `SURE_BIN` is removed and `HOME` points at an empty directory,
/// which is what makes `$HOME/.cargo/bin/sure` unreachable. The other two are
/// not: `PATH`, which this checks entry by entry, and the two absolute
/// fallbacks `integrations/*/scripts/sure-hook.sh` names in its `for p in` loop,
/// which no environment variable can move. A machine that has a `sure` in one
/// of them fails here, saying so, rather than passing a test that proved
/// nothing — and the `.ps1` launchers' `%LOCALAPPDATA%\SURE\bin\sure.exe` needs
/// no check because `LOCALAPPDATA` is the empty directory.
#[cfg(unix)]
fn assert_sure_cannot_be_found(path: &str) {
    let from_path = path
        .split(':')
        .map(|directory| Path::new(directory).join("sure"));
    let from_the_launchers = ["/opt/homebrew/bin/sure", "/usr/local/bin/sure"]
        .into_iter()
        .map(PathBuf::from);
    for candidate in from_path.chain(from_the_launchers) {
        assert!(
            !candidate.is_file(),
            "this machine has a `sure` at {}, and the missing-binary test needs an environment \
             where no `sure` can be found: the `.sh` launchers reach that path through `PATH` or \
             through the absolute fallbacks, neither of which the test can take away. Nothing in \
             this repository installs there; something else on this machine did. Move it, or point \
             UNIX_UTILITY_PATH at directories that exist on this platform and hold no `sure`.",
            candidate.display()
        );
    }
}

/// One harness's event, in the shape that harness sends it.
///
/// The stand-in does not parse it, and would not care if it did: what the tests
/// below need from the payload is that it is one non-empty line, so that a
/// launcher which dropped it is distinguishable from one that forwarded it.
/// Built with `serde_json` rather than by hand so that the quoting is the
/// library's problem on every platform.
fn an_event(harness: &str) -> String {
    let event = match harness {
        // Codex's payload names its own event, which is why its manifest passes
        // no event-kind argument.
        "codex" => serde_json::json!({
            "hook_event_name": "PreToolUse",
            "session_id": "p13t007",
            "cwd": ".",
            "model": "gpt-5-codex",
            "tool_name": "Bash",
            "tool_input": {"command": "echo hello"},
        }),
        _ => serde_json::json!({
            "event": "PreToolUse",
            "harness_session_id": "p13t007",
            "timestamp_utc": "2026-09-19T00:00:00Z",
            "source": harness,
            "tool": "Bash",
            "args": {"command": "echo hello"},
        }),
    };
    event.to_string()
}

/// The event-kind argument the package's own manifest passes, if it passes one.
///
/// Read off `hooks/hooks.json` in
/// [`the_table_covers_every_wired_event_and_matches_the_tree`], which is the
/// test that would notice a manifest changing its mind.
fn event_kind(harness: &str) -> Option<&'static str> {
    match harness {
        // `integrations/codex/hooks/hooks.json` runs the launcher with no
        // argument: the payload names its own event.
        "codex" => None,
        _ => Some("pre-tool-use"),
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// --- SURE missing: the launcher must not block the session --------------

/// What one launcher must write when SURE cannot be resolved at all.
struct MissingBin {
    /// Fragments that must appear on stderr.
    stderr_has: Vec<&'static str>,
    /// Fragments that must not appear on stderr.
    stderr_lacks: Vec<&'static str>,
    /// Whether the launcher must write *nothing at all* to stderr.
    ///
    /// Distinct from `stderr_lacks`, which only rules out the words that would
    /// be read as an answer. One launcher's missing-binary branch is documented
    /// as saying nothing whatsoever, and a weaker assertion would let that
    /// change while the note it must not write is replaced by a different one.
    silent: bool,
}

/// The expectation for one launcher, by harness and by which stream it uses.
///
/// Cursor's POSIX launcher is the one that writes **nothing**: its missing-binary
/// branch consumes the payload and exits 0 with both streams empty, which is a
/// different failure semantics from every other launcher here and is exactly the
/// cell a reader of the table needs. It is also the cell a user who set Cursor's
/// `failClosed` would notice: Cursor's hooks page lists "no output" among the
/// failures that flag turns into a block. That page is cited, per harness and
/// per event, in `docs/integrations/HOOK_FAILURE_SEMANTICS.md`; no test here
/// observes Cursor, and none of them should be read as if it did.
fn missing_bin(harness: &str, script: &Path) -> MissingBin {
    let posix = script.extension().and_then(|e| e.to_str()) == Some("sh");
    let named = |source: &'static str| MissingBin {
        stderr_has: vec!["SURE binary not found", source],
        stderr_lacks: vec![],
        silent: false,
    };
    match (harness, posix) {
        ("cursor", true) => MissingBin {
            stderr_has: vec![],
            stderr_lacks: vec!["SURE binary not found", "decision"],
            silent: true,
        },
        ("codex", _) => MissingBin {
            stderr_has: vec!["SURE binary not found", "codex"],
            // The Codex launchers deliberately carry no `decision`: SURE never
            // ran, and a decision SURE did not make is not one the launcher may
            // invent.
            stderr_lacks: vec!["decision"],
            silent: false,
        },
        ("claude-code", _) => {
            let mut shape = named("claude-code");
            shape.stderr_has.push("decision");
            shape.stderr_has.push("allow");
            shape
        }
        ("cursor", false) => {
            let mut shape = named("cursor");
            shape.stderr_has.push("decision");
            shape.stderr_has.push("allow");
            shape
        }
        ("copilot", _) => {
            let mut shape = named("copilot");
            shape.stderr_has.push("decision");
            shape.stderr_has.push("allow");
            shape
        }
        (other, _) => panic!("no missing-binary expectation for {other}"),
    }
}

#[test]
fn no_launcher_blocks_the_session_when_sure_cannot_be_found() {
    // The failure this file exists for. A user who has not installed SURE, or
    // whose `PATH` does not reach it, must not lose their agent: every one of
    // these exits 0 so the harness proceeds, and the ones that say anything say
    // it on stderr, where it cannot be read as a decision by a harness that
    // parses stdout.
    let scratch = scratch("hook-missing-bin");
    let launchers = hook_launchers();
    assert!(
        !launchers.is_empty(),
        "no hook launcher was found for this platform, so this test proves nothing"
    );
    for (harness, script) in &launchers {
        assert!(
            script.is_file(),
            "{} ships no launcher at {}",
            harness,
            script.display()
        );
        let expectation = missing_bin(harness, script);
        let output = run_launcher(
            &scratch,
            script,
            None,
            event_kind(harness),
            &an_event(harness),
        );
        let stdout = text(&output.stdout);
        let stderr = text(&output.stderr);

        assert_eq!(
            output.status.code(),
            Some(0),
            "{harness}: a missing SURE binary blocked the session. A hook that cannot run must \
             leave the agent working, and this one returned {:?} with stderr:\n{stderr}",
            output.status.code()
        );
        assert!(
            stdout.is_empty(),
            "{harness}: the launcher wrote to stdout when SURE was missing, and stdout is what a \
             harness reads as SURE's answer:\n{stdout}"
        );
        for fragment in &expectation.stderr_has {
            assert!(
                stderr.contains(fragment),
                "{harness}: the note about a missing SURE binary does not say {fragment:?}:\n{stderr}"
            );
        }
        for fragment in &expectation.stderr_lacks {
            assert!(
                !stderr.contains(fragment),
                "{harness}: the launcher wrote {fragment:?} when SURE never ran:\n{stderr}"
            );
        }
        if expectation.silent {
            assert!(
                stderr.is_empty(),
                "{harness}: this launcher's missing-binary branch is documented as writing \
                 nothing at all, and it wrote:\n{stderr}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

// --- SURE ran: the launcher must relay what it answered ------------------

/// One launcher, one status, relayed unchanged.
fn assert_relayed(harness: &str, script: &Path, scratch: &Path, code: u8) {
    let bin = stand_in(scratch, code);
    let output = run_launcher(
        scratch,
        script,
        Some(&bin),
        event_kind(harness),
        &an_event(harness),
    );
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(i32::from(code)),
        "{harness}: SURE answered {code} and the launcher returned {:?}. A status the launcher \
         rewrites is a status the harness never sees.\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains(&format!("stub-out {code}")),
        "{harness}: what SURE wrote to stdout did not reach the harness:\n{stdout}"
    );
    assert!(
        stderr.contains(&format!("stub-err {code}")),
        "{harness}: what SURE wrote to stderr did not reach the harness:\n{stderr}"
    );
}

#[test]
fn a_launcher_relays_a_sure_run_that_succeeded() {
    let scratch = scratch("hook-relay-0");
    for (harness, script) in hook_launchers() {
        assert_relayed(&harness, &script, &scratch, 0);
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_launcher_relays_a_blocked_request_status_and_the_frame_that_says_so() {
    // Exit 1 is the status SURE returns for a `block` decision
    // (`crates/sure-cli/src/report.rs`, the `HookDecision` arm). Whether a
    // harness treats 1 as a refusal is a separate question with its own column
    // in the table — what is asserted here is only that the launcher does not
    // swallow it on the way.
    let scratch = scratch("hook-relay-1");
    for (harness, script) in hook_launchers() {
        assert_relayed(&harness, &script, &scratch, 1);
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_launcher_relays_the_status_of_a_run_that_could_not_finish() {
    // Exit 5 is `exit::FAILED`: SURE tried and did not finish. It is not a
    // refusal, and the launcher must not turn it into one — nor into a 0 a
    // harness would read as "SURE looked and said yes".
    let scratch = scratch("hook-relay-5");
    for (harness, script) in hook_launchers() {
        assert_relayed(&harness, &script, &scratch, 5);
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

// --- the table, kept true against the tree -------------------------------

/// Every hook launcher the tree ships, as `(harness, path)`, for the platform
/// this test is compiled on.
///
/// `hook_launchers` cannot be reused for the table check: that one answers for
/// the platform, and the table has to answer for the tree.
fn hook_launchers_named(file: &str) -> Vec<(String, PathBuf)> {
    let integrations = sure_testkit::repository_root().join("integrations");
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(&integrations)
        .expect("the integrations directory is readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let harness = entry.file_name().to_string_lossy().into_owned();
            let script = entry.path().join("scripts").join(file);
            script.is_file().then_some((harness, script))
        })
        .collect();
    found.sort();
    found
}

/// One row of the table.
struct Row {
    harness: String,
    event: String,
    evidence: String,
    semantics: String,
    source: String,
    line: usize,
}

/// The rows of the per-event table, read out of the page.
///
/// A hand-rolled reader rather than a Markdown parser, and it is strict on
/// purpose: the header must name the columns, the table must be one contiguous
/// run of rows, and every row must have a cell in every column. A table that
/// stopped being a table is the failure this catches — the alternative, a
/// reader that quietly found no rows, would let every assertion below pass over
/// an empty list.
fn rows(text: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut header_seen = false;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if header_seen && !line.starts_with('|') {
            // The table ended. Anything after it belongs to another table, and
            // reading one of those as rows would check the wrong text.
            break;
        }
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`').trim().to_owned())
            .collect();
        if !header_seen {
            if cells == COLUMNS {
                header_seen = true;
            }
            continue;
        }
        if cells
            .iter()
            .all(|cell| cell.chars().all(|c| c == '-' || c == ':'))
        {
            continue;
        }
        assert_eq!(
            cells.len(),
            COLUMNS.len(),
            "{}:{} has {} cells where the table declares {} columns",
            TABLE,
            index + 1,
            cells.len(),
            COLUMNS.len()
        );
        rows.push(Row {
            harness: cells[0].clone(),
            event: cells[1].clone(),
            evidence: cells[4].clone(),
            semantics: cells[5].clone(),
            source: cells[6].clone(),
            line: index + 1,
        });
    }
    assert!(
        header_seen,
        "{TABLE} has no table with the columns {COLUMNS:?}. The per-event failure table is what \
         this file checks; without it every harness and event it is supposed to answer for is \
         unanswered."
    );
    rows
}

/// Every Rust source file under `crates/`, concatenated.
///
/// Used only by the check that an evidence cell names a test that exists. It is
/// deliberately the whole workspace rather than the file the row is about: a
/// test may live in the crate that decides or in the testkit that runs the
/// launcher, and the cell is allowed to name either.
fn workspace_sources() -> String {
    let mut text = String::new();
    let mut stack = vec![sure_testkit::repository_root().join("crates")];
    while let Some(directory) = stack.pop() {
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("cannot read a directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
                text.push('\n');
            }
        }
    }
    // A silent failure here would be the worst kind: every name below would be
    // "not found" and the check would look like it worked.
    assert!(
        text.len() > 100_000,
        "the Rust source scan found {} bytes under crates/, which is too little to be the \
         workspace. The evidence check cannot run against it.",
        text.len()
    );
    text
}

/// The bare test names in an evidence cell: backticked tokens with nothing in
/// them that would make them a path.
///
/// `hook.rs:295-300` and `pre-tool-use` are citations of their own and are not
/// checked here — the first is checked by review, the second is the argument a
/// manifest passes. A token that is one lowercase identifier, `like_this`, is
/// claiming to be the name of a test, and that claim is checkable.
fn bare_names(cell: &str) -> Vec<String> {
    fn is_a_name(token: &str) -> bool {
        token.len() >= 8
            && token.starts_with(|character: char| character.is_ascii_lowercase())
            && token.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
    }

    let mut names = Vec::new();
    // The reader strips the backticks that wrapped a whole cell, so a cell that
    // is exactly one name arrives as one name.
    if is_a_name(cell) {
        names.push(cell.to_owned());
    }
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        if is_a_name(&after[..close]) {
            names.push(after[..close].to_owned());
        }
        rest = &after[close + 1..];
    }
    names
}

/// The event names one package's manifest wires.
fn wired_events(harness: &str) -> Vec<String> {
    let path = sure_testkit::repository_root()
        .join("integrations")
        .join(harness)
        .join("hooks")
        .join("hooks.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let manifest: Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()));
    let hooks = manifest
        .get("hooks")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{} declares no hooks object", path.display()));
    assert!(
        !hooks.is_empty(),
        "{} wires no events, so a table row for it would be checked against nothing",
        path.display()
    );
    let mut events: Vec<String> = hooks.keys().cloned().collect();
    events.sort();
    events
}

/// Every package in the tree that ships a hook launcher, on any platform.
fn packages_with_hook_launchers() -> Vec<String> {
    let mut packages: Vec<String> = hook_launchers_named("sure-hook.ps1")
        .into_iter()
        .chain(hook_launchers_named("sure-hook.sh"))
        .map(|(harness, _)| harness)
        .collect();
    packages.sort();
    packages.dedup();
    packages
}

#[test]
fn the_table_covers_every_wired_event_and_matches_the_tree() {
    // The acceptance, made checkable: *explicit for each harness/event*. A
    // package that wires a new event, or a new package that ships a launcher,
    // fails here until the table says what that cell does when SURE cannot
    // answer — which is the state of the tree this task is about, not a style
    // preference.
    let text = std::fs::read_to_string(sure_testkit::repository_root().join(TABLE))
        .unwrap_or_else(|error| panic!("cannot read {TABLE}: {error}"));
    let rows = rows(&text);

    let packages = packages_with_hook_launchers();
    assert!(
        packages.len() >= 4,
        "expected four packages with a hook launcher, found {packages:?}"
    );

    let mut expected: Vec<(String, String)> = Vec::new();
    for harness in &packages {
        for event in wired_events(harness) {
            expected.push((harness.clone(), event));
        }
    }

    // Read once, before the loop: this is every `.rs` file in the workspace, and
    // the point of the check below is that a name in the table is a name that is
    // in the tree.
    let sources = workspace_sources();

    for (harness, event) in &expected {
        let row = rows
            .iter()
            .find(|row| &row.harness == harness && &row.event == event)
            .unwrap_or_else(|| {
                panic!(
                    "{TABLE} says nothing about `{harness}` event `{event}`, which \
                     `integrations/{harness}/hooks/hooks.json` wires. Every wired event needs a \
                     failure semantics of its own; the table has {} row(s) for this harness.",
                    rows.iter().filter(|row| &row.harness == harness).count()
                )
            });
        assert!(
            SEMANTICS.contains(&row.semantics.as_str()),
            "{TABLE}:{} gives `{harness}` event `{event}` the semantics {:?}, which is not one of \
             {SEMANTICS:?}. The cell has to say which way it falls, or say that this repository \
             does not know.",
            row.line,
            row.semantics
        );
        assert!(
            row.source.len() > 2,
            "{TABLE}:{} gives no source for `{harness}` event `{event}`. Every cell is either \
             backed by something in this repository or cited to something outside it.",
            row.line
        );
        assert!(
            !bare_names(&row.evidence).is_empty(),
            "{TABLE}:{} gives `{harness}` event `{event}` no evidence in this repository. The \
             evidence cell names the test that pins what SURE answers, or the branch in the code \
             that decides it; {:?} names neither.",
            row.line,
            row.evidence
        );
        for name in bare_names(&row.evidence) {
            assert!(
                sources.contains(&name),
                "{TABLE}:{} gives `{harness}` event `{event}` the evidence `{name}`, which appears \
                 in no Rust file under crates/. A cell that names a test nobody wrote reads like \
                 evidence and is not: write the test, or cite the branch in the code that decides \
                 and say what would have to change for it to be checked.",
                row.line
            );
        }
        if row.semantics == "cannot confirm" {
            assert!(
                row.source.contains("would confirm"),
                "{TABLE}:{} says this repository cannot confirm the semantics of `{harness}` event \
                 `{event}` and does not say what would settle it. An unverified cell that does not \
                 name its own remedy is a dead end: {:?}",
                row.line,
                row.source
            );
        }
    }

    // And the other direction: a row for an event nothing wires is a table that
    // has drifted away from the packages it describes.
    for row in &rows {
        assert!(
            packages.contains(&row.harness),
            "{TABLE}:{} names `{}`, which ships no hook launcher",
            row.line,
            row.harness
        );
        assert!(
            expected.contains(&(row.harness.clone(), row.event.clone())),
            "{TABLE}:{} names `{}` event `{}`, which no manifest wires",
            row.line,
            row.harness,
            row.event
        );
    }
}

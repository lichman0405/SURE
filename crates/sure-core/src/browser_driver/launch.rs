//! Starting the browser, and finding the port it chose.
//!
//! This is the only file in the product that starts a browser, and it is the
//! fourth entry in `crates/sure-core/tests/spawn_sites.rs`'s census — which is
//! the check that exists so that a new way for SURE to run something cannot be
//! added without somebody reading the paragraph above the list.
//!
//! # Why the browser is started directly rather than through the runner
//!
//! [`crate::process`] is how SURE runs *a project's* commands, and the door into
//! it is `Enforcement::admitted`, which classifies a program by name and treats
//! one it does not recognise as capable of anything. A browser is not in that
//! table, so a browser cannot be started through it — correctly, because the
//! runner's contract is about the project's code and this is not the project's
//! code.
//!
//! **The browser is SURE's own tool, chosen by SURE from a table compiled into
//! this binary.** That is the same shape as
//! [`crate::fingerprint::git`](crate::fingerprint::git), which holds a program
//! name rather than calling for `git` where it is needed, and for the same
//! reason. What follows from that is the security property worth stating: the
//! address SURE opens comes from the project, and **nothing else does**.
//!
//! # Where the address is, and why not on the command line
//!
//! `--remote-debugging-port=0` asks the operating system for a free port, and
//! the browser writes the port it got into `DevToolsActivePort` inside its
//! profile directory. SURE reads it from there.
//!
//! **The page's address is never a command-line argument.** It is sent later, as
//! a `Page.navigate` command over the debugging connection, so there is no
//! argument vector for a project-supplied path to be injected into — no quoting
//! rule to get right, no `--` to place, and nothing that behaves differently
//! because a path began with a dash. `CLAUDE.md`'s rule about preferring typed
//! arguments to shell strings is satisfied here by there being no string.
//!
//! # The profile directory
//!
//! A browser started with remote debugging **refuses to use a profile that is
//! already the default one**, which is what stops a page from reaching the
//! debugging port of the browser a person is using. SURE creates a private
//! directory under the system temporary directory for each run, starts the
//! browser against that, and removes it afterwards. Nothing of the user's is
//! opened, read or written, and a run leaves no profile behind for the next one
//! to inherit.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often the profile directory is looked at while the browser starts.
///
/// The file appears once and is then read; polling is what bounds the wait by
/// time rather than by a guess about how long a browser takes to start on a
/// machine this build has never run on.
const POLL: Duration = Duration::from_millis(25);

/// How much of what the browser said on its error stream is kept.
///
/// **Bounded, because the stream is not.** A browser that fails to start may say
/// a sentence or may say a great deal, and the report quotes this rather than
/// the whole of it. Four kibibytes is several times the longest Chrome startup
/// complaint anybody has needed to read.
const COMPLAINT_KEPT: usize = 4096;

/// A browser that is running, and where to talk to it.
///
/// Dropping this stops the browser. That is not a convenience: the value is held
/// across the whole look, and a path that returned early leaving a browser
/// running would leak a process — and a bound port — per checked project.
#[derive(Debug)]
pub struct Launched {
    child: Child,
    profile: PathBuf,
    port: u16,
    browser_path: String,
    /// Whether the child has already been reaped, so that `Drop` does not try to
    /// stop a process whose identifier the operating system may have handed to
    /// somebody else.
    reaped: bool,
}

/// Why a browser is not available to be driven.
///
/// Every variant ends as
/// [`AbsenceReason::DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart),
/// so none of them says anything about the project. They are separate variants
/// because the sentence a person reads differs, and because a test can tell them
/// apart without matching on prose.
#[derive(Debug)]
pub enum LaunchError {
    /// The operating system would not start the program.
    WouldNotStart {
        program: PathBuf,
        error: std::io::Error,
    },
    /// The program started and never said which port it was listening on.
    NeverOpened {
        program: PathBuf,
        waited: Duration,
        complaint: String,
    },
    /// The program started and then stopped before it said.
    Exited {
        program: PathBuf,
        status: String,
        complaint: String,
    },
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WouldNotStart { program, error } => write!(
                formatter,
                "{} could not be started: {error}",
                program.display()
            ),
            Self::NeverOpened {
                program,
                waited,
                complaint,
            } => write!(
                formatter,
                "{} ran for {} seconds without reporting a debugging port{}",
                program.display(),
                waited.as_secs_f32(),
                said(complaint)
            ),
            Self::Exited {
                program,
                status,
                complaint,
            } => write!(
                formatter,
                "{} stopped before reporting a debugging port ({status}){}",
                program.display(),
                said(complaint)
            ),
        }
    }
}

/// The browser's own words, if it said any, as a clause a sentence can end with.
fn said(complaint: &str) -> String {
    let trimmed = complaint.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        // Quoted rather than folded into the sentence, because these are the
        // program's words and not SURE's reading of them.
        format!(", and said: {trimmed:?}")
    }
}

impl std::error::Error for LaunchError {}

/// Starts `program` and waits for it to report the port it is listening on.
///
/// `budget` bounds the wait, and it is spent entirely if the browser never
/// answers: a caller that has already used part of its own time passes what is
/// left rather than a fresh allowance, so that one look cannot take longer than
/// the limit its caller set.
///
/// # Errors
///
/// [`LaunchError`], which the caller turns into
/// [`AbsenceReason::DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart).
pub fn start(program: &Path, budget: Duration) -> Result<Launched, LaunchError> {
    let profile = private_profile();
    std::fs::create_dir_all(&profile).map_err(|error| LaunchError::WouldNotStart {
        program: program.to_path_buf(),
        error,
    })?;

    let mut child = match Command::new(program)
        .args(arguments(&profile))
        // The browser is run from its own private directory rather than from
        // wherever SURE was started. A process's working directory is a handle
        // on that directory, and on Windows a handle held by a program the
        // project did not write is still a handle that can stop the project's
        // directory being renamed or removed.
        .current_dir(&profile)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            // The directory was made before the program was tried, so a program
            // that cannot start would otherwise leave one behind on every call.
            // Nothing has run, so nothing holds it open and this succeeds.
            let _ = std::fs::remove_dir_all(&profile);
            return Err(LaunchError::WouldNotStart {
                program: program.to_path_buf(),
                error,
            });
        }
    };

    // The buffer the draining thread writes into. **Nothing reads it after a
    // successful start**, and it is kept for the whole life of the child on
    // purpose: the thread ends when the browser's error stream does, and until
    // then its only job is to be somewhere the pipe can empty into. A pipe
    // nobody reads fills, and a browser that blocks writing a warning it did not
    // ask anybody to read would stop answering on the debugging port.
    let complaint = Arc::new(Mutex::new(String::new()));
    if let Some(stream) = child.stderr.take() {
        drain_into(stream, Arc::clone(&complaint));
    }

    let deadline = Instant::now() + budget;
    loop {
        match read_active_port(&profile) {
            Some((port, browser_path)) => {
                return Ok(Launched {
                    child,
                    profile,
                    port,
                    browser_path,
                    reaped: false,
                });
            }
            None => {
                // A browser that has already stopped will not write the file,
                // and waiting out the whole budget to say so wastes the caller's
                // time to learn something already known.
                if let Ok(Some(status)) = child.try_wait() {
                    let complaint = complaint_text(&complaint);
                    let _ = std::fs::remove_dir_all(&profile);
                    return Err(LaunchError::Exited {
                        program: program.to_path_buf(),
                        status: status.to_string(),
                        complaint,
                    });
                }
            }
        }
        if Instant::now() >= deadline {
            let complaint = complaint_text(&complaint);
            let mut child = child;
            let _ = crate::process::terminate::stop(&mut child);
            let _ = std::fs::remove_dir_all(&profile);
            return Err(LaunchError::NeverOpened {
                program: program.to_path_buf(),
                waited: budget,
                complaint,
            });
        }
        std::thread::sleep(POLL);
    }
}

/// The arguments the browser is started with, in full.
///
/// **This list is the configuration the mechanism was verified against**, and it
/// is written out rather than trimmed because a flag removed here is a
/// configuration that has not been tested. Each one earns its place:
///
/// - `--headless=new` — no window. A check that opened one would take over the
///   screen of the person running it, and `new` is the mode that renders with
///   the real engine rather than the old headless shell.
/// - `--remote-debugging-port=0` — the whole point; zero asks for a free port.
/// - `--user-data-dir` — the private profile the module documentation explains.
/// - `--no-first-run`, `--no-default-browser-check` — without these a fresh
///   profile starts a first-run flow that can block before the port is written.
///   They are not cosmetic: they are the difference between a browser that
///   answers and one that waits for a person who is not there.
/// - `--disable-gpu` — a machine running this may have no usable graphics
///   device, and a browser that stalls on one never writes the port file.
/// - `--disable-extensions` — the profile is new, so there is nothing to
///   disable; it is here because it is part of the verified configuration and
///   removing a flag from a verified configuration is a change that has not
///   been tested.
///
/// **`--no-sandbox` is deliberately absent.** It is the flag that makes a
/// Chromium start in containers and on CI images whose kernel restricts user
/// namespaces, and it is the flag that is reached for first when a browser will
/// not start on a build machine. It is not here because **the page this browser
/// opens is written by the project being checked** — it is the most untrusted
/// content in the whole of SURE — and the sandbox is what stops that page
/// reaching the rest of the machine. A browser that will not start without it is
/// reported as
/// [`DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart)
/// and the check is skipped, which is a smaller loss than running a project's
/// JavaScript without the boundary that exists to contain it.
///
/// The address is not in this list. See the module documentation.
fn arguments(profile: &Path) -> Vec<std::ffi::OsString> {
    let mut arguments: Vec<std::ffi::OsString> = [
        "--headless=new",
        "--remote-debugging-port=0",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-gpu",
        "--disable-extensions",
    ]
    .iter()
    .map(std::ffi::OsString::from)
    .collect();
    let mut data_dir = std::ffi::OsString::from("--user-data-dir=");
    data_dir.push(profile.as_os_str());
    arguments.push(data_dir);
    // The page this browser shows until SURE tells it where to go. Not a URL
    // from the project, so that the first thing the browser loads is nothing.
    arguments.push(std::ffi::OsString::from("about:blank"));
    arguments
}

/// A directory of this run's own, under the system temporary directory.
///
/// The process identifier and a counter, so that two runs in one process — and
/// two tests in one test binary, which is where the second one matters — never
/// share a profile. A shared profile is not a hypothetical: a browser refuses to
/// start against a profile another browser is using, so the failure would be a
/// check that could not run depending on what ran beside it.
fn private_profile() -> PathBuf {
    static RUNS: AtomicU32 = AtomicU32::new(0);
    let run = RUNS.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("sure-browser-{}-{run}", std::process::id()))
}

/// The port and browser path the browser wrote, if it has written them.
///
/// The file's first line is the port and its second is the path of the
/// browser-wide debugging endpoint. **Both are read**, and the second is not
/// decoration: it is the only way to reach the endpoint that can list targets,
/// which is how a page is found.
///
/// A file that exists but does not yet hold two parseable lines returns `None`
/// rather than an error, because that is what a file **being written** looks
/// like — the browser creates it before it fills it in, and a reader that
/// treated the moment in between as a failure would fail on a fast machine and
/// pass on a slow one.
fn read_active_port(profile: &Path) -> Option<(u16, String)> {
    let text = std::fs::read_to_string(profile.join("DevToolsActivePort")).ok()?;
    let mut lines = text.lines();
    let port = lines.next()?.trim().parse::<u16>().ok()?;
    let path = lines.next()?.trim().to_owned();
    if !path.starts_with('/') {
        return None;
    }
    Some((port, path))
}

/// Reads a stream to its end into a shared, bounded buffer, on its own thread.
///
/// **A thread, because the alternative deadlocks.** A child's error stream is a
/// pipe with a fixed capacity: a program that writes more than that blocks until
/// somebody reads, and a parent that waits for the program to exit before
/// reading waits for a program that is waiting for it. The thread ends when the
/// stream does, which is when the browser exits.
///
/// The buffer stops growing at [`COMPLAINT_KEPT`], and says so — a report that
/// quoted the first four kibibytes of a very long complaint without saying it
/// had stopped would be a quote that looked complete.
fn drain_into(mut stream: impl std::io::Read + Send + 'static, into: Arc<Mutex<String>>) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(read) => {
                    let Ok(mut kept) = into.lock() else {
                        return;
                    };
                    if kept.len() >= COMPLAINT_KEPT {
                        continue;
                    }
                    let room = COMPLAINT_KEPT - kept.len();
                    let take = room.min(read);
                    kept.push_str(&String::from_utf8_lossy(&buffer[..take]));
                    if take < read {
                        kept.push_str(" […]");
                    }
                }
            }
        }
    });
}

/// What the browser said, or an empty string.
fn complaint_text(complaint: &Arc<Mutex<String>>) -> String {
    complaint
        .lock()
        .map(|kept| kept.clone())
        .unwrap_or_default()
}

impl Launched {
    /// The path of the browser-wide debugging endpoint, beginning with a slash.
    #[must_use]
    pub fn browser_path(&self) -> &str {
        &self.browser_path
    }

    /// Where the debugging connection is made.
    #[must_use]
    pub fn address(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, self.port))
    }

    /// Waits up to `timeout` for the browser to exit on its own.
    ///
    /// Returns whether it did. This is the graceful half of stopping: a browser
    /// asked to close over the debugging connection shuts itself down and
    /// removes its own profile, and a caller that killed it instead would be
    /// killing something that was already leaving.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    self.reaped = true;
                    return true;
                }
                Ok(None) => {}
                Err(_) => return false,
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL);
        }
    }
}

impl Drop for Launched {
    fn drop(&mut self) {
        if !self.reaped {
            // The tree, not the process: a browser has children, and the
            // operating system's own tree stop is what reaches them. The result
            // is deliberately not unwrapped — a drop cannot report — and the
            // fallback below runs whether or not it worked.
            let _ = crate::process::terminate::stop(&mut self.child);
            let _ = self.child.wait();
            self.reaped = true;
        }
        // Best effort, and after the browser has gone: on Windows a directory a
        // running program has open cannot be removed, so a failure here means
        // the browser is still holding it and not that the removal was wrong.
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// **`--no-sandbox` is not in the argument vector**, which is a security
    /// decision this test holds in place rather than a formatting one.
    ///
    /// The page this browser opens is written by the project being checked, so
    /// the sandbox is the boundary between that page and the machine. The
    /// pressure to add the flag is real — it is what makes a Chromium start on
    /// CI images and in containers — and it is the pressure this test is for.
    #[test]
    fn the_sandbox_is_never_disabled() {
        let arguments = arguments(Path::new("/tmp/a-profile"));
        let flattened: Vec<String> = arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        for forbidden in [
            "--no-sandbox",
            "--disable-web-security",
            "--allow-running-insecure-content",
        ] {
            assert!(
                !flattened.iter().any(|argument| argument == forbidden),
                "{forbidden} was passed, and the page this browser opens is the project's"
            );
        }
        assert!(
            flattened
                .iter()
                .any(|argument| argument == "--headless=new"),
            "the browser would open a window"
        );
        assert!(
            flattened
                .iter()
                .any(|argument| argument.starts_with("--user-data-dir=")),
            "the browser would use the profile of the person running SURE"
        );
    }

    /// **The page's address is not among the arguments**, because it is sent
    /// later as a protocol command.
    ///
    /// What that buys is stated in the module documentation: there is no
    /// argument vector for a project-supplied path to be injected into. The
    /// assertion is that the only thing in the vector that could be a URL is the
    /// constant this file wrote.
    #[test]
    fn the_only_address_on_the_command_line_is_the_one_this_file_wrote() {
        let arguments = arguments(Path::new("/tmp/a-profile"));
        let last = arguments.last().expect("there is a last argument");
        assert_eq!(last.to_string_lossy(), "about:blank");
        for argument in &arguments {
            let text = argument.to_string_lossy();
            assert!(
                !text.starts_with("http://") && !text.starts_with("https://"),
                "{text} is an address from somewhere other than this file"
            );
        }
    }

    /// **Two runs never share a profile**, which is what makes it safe for the
    /// tests in one binary to run at the same time.
    #[test]
    fn a_profile_is_this_run_s_own() {
        let first = private_profile();
        let second = private_profile();
        assert_ne!(first, second, "two runs were given one profile");
        assert!(first.is_absolute(), "{} is not absolute", first.display());
    }

    /// The file the browser writes, read from a directory this test wrote —
    /// including the two states that are not a port.
    ///
    /// **A half-written file is `None` and not an error**, because that is what
    /// the file looks like between being created and being filled in, and a
    /// reader that called that a failure would fail on whichever machine won the
    /// race.
    #[test]
    fn the_port_file_is_read_only_when_it_holds_both_lines() {
        let directory = std::env::temp_dir().join(format!("sure-port-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        let file = directory.join("DevToolsActivePort");

        std::fs::write(&file, "12345\n/devtools/browser/abc\n").expect("written");
        assert_eq!(
            read_active_port(&directory),
            Some((12345, String::from("/devtools/browser/abc")))
        );

        std::fs::write(&file, "12345\n").expect("written");
        assert_eq!(read_active_port(&directory), None, "a half-written file");

        std::fs::write(&file, "not a port\n/devtools/browser/abc\n").expect("written");
        assert_eq!(
            read_active_port(&directory),
            None,
            "a first line that is not a port"
        );

        std::fs::write(&file, "12345\nnot-a-path\n").expect("written");
        assert_eq!(
            read_active_port(&directory),
            None,
            "a second line that is not a path"
        );

        std::fs::remove_file(&file).expect("removed");
        assert_eq!(read_active_port(&directory), None, "no file at all");

        std::fs::remove_dir_all(&directory).expect("removed");
    }

    /// **A program that cannot be started is reported, not panicked on**, and
    /// the report names the program.
    #[test]
    fn a_program_that_does_not_exist_is_an_error_a_caller_can_read() {
        let missing = std::env::temp_dir().join("sure-there-is-no-such-browser-anywhere");
        let error = start(&missing, Duration::from_millis(500))
            .expect_err("a program that does not exist cannot be started");
        assert!(
            matches!(error, LaunchError::WouldNotStart { .. }),
            "{error:?}"
        );
        assert!(
            error
                .to_string()
                .contains("sure-there-is-no-such-browser-anywhere"),
            "the report does not name the program: {error}"
        );
    }
}

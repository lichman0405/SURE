//! Starting the project up, asking it something, and stopping it.
//!
//! `P5-T001` decided *what* SURE would start and handed back a [`RuntimeProbe`]
//! whose reason names the command the project's own manifest declares. This
//! module is what does it: start that command, watch a **startup window**, ask
//! the service a question if there is one to ask, stop the service, and turn all
//! of that into one [`CheckResult`]. The task's acceptance is two sentences —
//! *"Supported service can be started/probed/terminated"* and *"Startup failure
//! remains explicit"* — and the second is the harder of the two, because a start
//! that failed is exactly the case where a check quietly becomes a pass.
//!
//! # The chain, and where each link comes from
//!
//! ```text
//! ProbePlan::of        runtime_probes.rs   what SURE would start, and why
//! PermissionPlan::add  consent.rs          the program, its arguments, and the decision
//! Enforcement::of      enforce.rs          which of those may run
//! Supervisor::start    service.rs          the process, and stopping it
//! StartSmoke::run      here                the window, the question, the stop, the verdict
//! ```
//!
//! [`StartSmoke::of`] takes an [`Enforcement`] and not a command, which is
//! `enforce.rs`'s own rule applied one level up — *a runner wired into this
//! product must take what it launches from `Enforcement::admitted`*. The
//! constructor looks the admitted command up **by the probe's own check id** and
//! refuses to build anything when there is none, so a smoke check cannot run a
//! command that a mode stopped, and cannot run a command that was planned for a
//! different check. What a mode stopped is answered by [`Enforcement::stopped`],
//! which is the plan's own `Skipped` result and not a second one invented here.
//!
//! **The split is deliberately not here.** Turning a declared line — `npm start`
//! — into a program and an argument vector is the executor's work, one step
//! before this module, for the same reason [`crate::service`] does not do it: a
//! second place that knew how a declared command becomes a program would be a
//! second answer to *what would SURE run*, and the first answer is the one the
//! user consented to. So this module runs what was decided and nothing else.
//!
//! # The startup window, which is a new thing and wants arguing for
//!
//! [`Supervisor`] has no readiness and says so in its own words: `Limits::timeout`
//! is the **whole-life budget** of the service, and *"reporting readiness from
//! the fact that a process exists is the false green this product is built
//! against"*. So the question *has this come up yet* has to be asked by
//! somebody, and this module is that somebody. The window is how long a service
//! is given to still be running: one that is alive when the window closes did
//! not fail to start, and one that ended inside it did.
//!
//! **The window is not a readiness test and nothing here pretends it is.** A
//! service that stays up is not a service that works — `"start": "sleep 600"`
//! stays up for ten minutes and answers nothing — which is why a pass here
//! needs a question to have been asked *and answered*, and why a service that
//! stayed up with nothing to ask is a [`CheckStatus::Warning`] rather than a
//! pass. On a critical check a warning is `CriticalState::Passed`, so it can
//! degrade an aggregate to `needs_attention` and can never produce a green.
//!
//! **Two budgets, and neither is rounded.** A zero window is refused
//! ([`LimitsError::ZeroWindow`]) for `browser::Limits`' own reason: a budget of
//! zero is a check that cannot fail by accident, and the honest answer to *I
//! want no time at all* is a refusal rather than a rounding. A window that is
//! **not shorter than the service's whole-life budget** is refused too
//! ([`LimitsError::WindowOutlastsTheBudget`]): the budget is measured from the
//! start and the window from the moment the start returned, so the service would
//! be stopped by its own deadline before the window ever closed, and SURE would
//! never get to ask its question. That is a check that cannot pass, and a check
//! that cannot pass is one somebody has to be told about rather than one to
//! discover from a warning.
//!
//! # The verdict table, which is the whole of what this module claims
//!
//! | What happened | Status | Why |
//! | --- | --- | --- |
//! | it ended by itself, inside the window or after it | `fail` | the thing SURE was going to ask is gone |
//! | it outlived the window and there was a question | the probe's own status | `pass`, `fail`, `unknown` or `error`, from `probe.rs`'s table |
//! | it outlived the window and there was nothing to ask | `warning` | it came up; whether the feature works is not known |
//! | the program never started | `error` | nothing ran, and nothing about the project was learned |
//! | SURE cannot say what the run did, or could not stop it | `error` | a process SURE cannot vouch for is not a process it may report on |
//!
//! **Ending by itself is a failure whether or not the window had closed**, and
//! the two sentences differ only in which fact they lead with. A service that
//! exits is not a service, whatever it printed on the way out, and `"start"` is
//! the one script whose whole meaning is *this keeps running*.
//!
//! **A question is only asked while the process SURE started is still running.**
//! Not after it ended, and that is not a shortcut: a port answering after SURE's
//! own process is gone may be answering for something SURE did not start — a
//! leftover from a previous run, another server on the machine, or a child the
//! start script detached. A pass built on that would be a claim about a service
//! SURE never saw come up, which is the false green this whole module is shaped
//! to avoid. So a service that ended is reported as ended, and SURE says which
//! question it did not get to ask.
//!
//! # What is not claimed here
//!
//! **Nothing about the feature.** A `pass` means the thing SURE started answered
//! the request SURE made, at the address SURE was told to look at. Whether the
//! answer was *right* is `P5-T003`'s route checks and `P5-T004`'s browser check,
//! and this module reads no body at all.
//!
//! **Nothing about what the service does while it runs.** It is a process, and
//! `docs/architecture/EXECUTION_SAFETY.md` is where the difference between *a
//! promise about what SURE starts* and *a boundary around what a started process
//! can reach* is stated.
//!
//! **Nothing about a second service.** One probe, one command, one process, one
//! question. Running several at once, sharing a port between them, and cleaning
//! up after a run that stopped early are `P5-T007`'s work.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sure_domain::evidence::EvidenceClass;
use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::status::{CheckResult, CheckStatus};

use crate::enforce::{AdmittedCommand, Enforcement};
use crate::probe::{self, Endpoint, Probe, ProbeOutcome};
use crate::process::{self, Outcome, Termination};
use crate::runtime_probes::{ProbeKind, RuntimeProbe};
use crate::service::{Service, Supervisor};

/// How many lines of a service's output a reason quotes.
///
/// The tail and not the head: a service that died said why on its way out, and
/// the first lines of a server's output are usually a banner. Three would be
/// `browser.rs`'s number for a list of problems; five is right here because a
/// stack trace's last useful line is rarely the last line, and `QUOTED_PROBLEMS`'
/// own comment says the count and the quote are different questions.
pub const QUOTED_LINES: usize = 5;

/// How many characters of that output a reason carries.
///
/// A reason is one line of a report and a program's output is unbounded, so the
/// quote is bounded twice — by lines and by characters — and the second bound is
/// what a minified build error with one enormous line hits first.
const QUOTED_CHARS: usize = 400;

/// How often a service is asked whether it is still running.
///
/// A poll rather than a notification, because [`Service::has_finished`] is the
/// only handle this build has on a run that has not ended. Milliseconds, because
/// the resolution is how coarse "it died inside the window" is: a service that
/// dies at 400 ms and a window of 500 ms are told apart with this, and 400 ms of
/// sleep would make the answer a coin toss.
const POLL: Duration = Duration::from_millis(20);

/// What one start smoke check is allowed to spend.
///
/// Three budgets and an address, and each of the four is a separate decision
/// because each belongs to a different thing: the window is this module's, the
/// service's budget is [`crate::process`]'s and covers the whole life of the
/// process, the request's is [`crate::probe`]'s and covers one exchange, and the
/// address is the project's — a `dev` script that prints *listening on 5173*
/// knows a port the manifest never writes down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    window: Duration,
    service: process::Limits,
    request: probe::Limits,
    endpoint: Option<Endpoint>,
}

impl Limits {
    /// The bounds for one smoke check, with nowhere to ask.
    ///
    /// `endpoint` is `None` for a service SURE has no address for, which is not
    /// a degraded check: it is the honest answer for a project whose port
    /// nothing declares, and the verdict is a warning that says so rather than a
    /// pass built on an assumption about which port a server picked.
    ///
    /// # Errors
    ///
    /// [`LimitsError::ZeroWindow`] if `window` is zero, and
    /// [`LimitsError::WindowOutlastsTheBudget`] if it is not shorter than
    /// `service`'s own timeout. Both are refusals rather than roundings, and the
    /// second is the one worth reading the module comment for: a window that
    /// cannot close before the budget runs out is a check that can never ask its
    /// question.
    pub fn new(
        window: Duration,
        service: process::Limits,
        request: probe::Limits,
        endpoint: Option<Endpoint>,
    ) -> Result<Self, LimitsError> {
        if window.is_zero() {
            return Err(LimitsError::ZeroWindow);
        }
        if window >= service.timeout() {
            return Err(LimitsError::WindowOutlastsTheBudget);
        }
        Ok(Self {
            window,
            service,
            request,
            endpoint,
        })
    }

    /// How long the service is given to still be running.
    #[must_use]
    pub const fn window(&self) -> Duration {
        self.window
    }

    /// The whole-life budget the service is started under.
    #[must_use]
    pub const fn service(&self) -> process::Limits {
        self.service
    }

    /// The bounds on the one exchange SURE may make.
    #[must_use]
    pub const fn request(&self) -> probe::Limits {
        self.request
    }

    /// Where SURE asks, when it has somewhere to ask.
    #[must_use]
    pub const fn endpoint(&self) -> Option<&Endpoint> {
        self.endpoint.as_ref()
    }
}

/// Why a smoke check could not be set up.
///
/// A budget this module is handed rather than a mode, a program or a
/// permission: those are decided before it and arrive as an [`Enforcement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitsError {
    /// The startup window is zero.
    ///
    /// Refused rather than rounded up, because rounding up would be this module
    /// choosing a duration the caller said was none — `browser::Limits`' own
    /// rule about a zero timeout, applied to the other budget here.
    ZeroWindow,
    /// The window is not shorter than the service's whole-life budget.
    ///
    /// The service would be stopped by its own deadline before the window
    /// closed, so SURE would never ask its question and the check could never
    /// pass. A configuration that produces no verdict wants to be a refusal, not
    /// a warning somebody has to work backwards from.
    WindowOutlastsTheBudget,
}

impl fmt::Display for LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWindow => formatter.write_str(
                "a startup window of zero is not a short window — a service would be given no \
                 time to come up at all",
            ),
            Self::WindowOutlastsTheBudget => formatter.write_str(
                "the startup window is not shorter than the service's own budget, so the service \
                 would be stopped by the deadline before SURE could ask it anything",
            ),
        }
    }
}

impl std::error::Error for LimitsError {}

/// Why a smoke check was not built.
///
/// Both variants are the same kind of statement — *there is nothing here for
/// this module to run* — and each names the thing that was wrong, because a
/// caller with a probe and an enforcement in hand has to be able to tell *you
/// gave me the browser probe* from *the plan did not admit this command*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeRefused {
    /// The probe is not one that starts a service.
    ///
    /// [`ProbeKind::Interface`] is the other kind, and it is `P5-T004`'s: its
    /// action is `BrowserProbe`, so what a user granted for it is the permission
    /// to look at a browser — not the permission to run the project's code,
    /// which is what starting one costs. Running a service under it would be
    /// doing something with a permission granted for something else.
    NotAServeProbe {
        /// The kind the probe actually is.
        kind: ProbeKind,
    },
    /// The enforcement holds no admitted command for this probe's check.
    ///
    /// Either the mode stopped it — in which case the result a report needs is
    /// in [`Enforcement::stopped`] and is already written — or the enforcement
    /// was built from a schedule this probe is not in, which is a caller's bug
    /// rather than a state to report on.
    NotAdmitted {
        /// The check that had no admitted command.
        check: CheckId,
    },
}

impl fmt::Display for SmokeRefused {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAServeProbe { kind } => write!(
                formatter,
                "this probe would {}, which is not the kind that runs the project's code, so there \
                 is nothing here to start",
                kind.plain_description()
            ),
            Self::NotAdmitted { check } => write!(
                formatter,
                "no command for check {check} was admitted, so there is nothing here to run"
            ),
        }
    }
}

impl std::error::Error for SmokeRefused {}

/// One serve probe, the command the mode admitted for it, and the bounds.
///
/// Built by [`StartSmoke::of`], which is the only constructor — the working
/// directory comes from the probe's component, the command comes from the
/// enforcement, and a type that could be assembled from a pair that disagree
/// would be a type that could start one component's command in another
/// component's directory.
#[derive(Debug)]
pub struct StartSmoke<'a> {
    /// The directory the service is started in: the project root joined with the
    /// probe's component, which is relative and empty for the root itself.
    directory: PathBuf,
    admitted: AdmittedCommand<'a>,
    fingerprint: FingerprintId,
    limits: Limits,
}

impl<'a> StartSmoke<'a> {
    /// A smoke check for `probe`, in the project rooted at `root`.
    ///
    /// The service runs in `root` joined with the probe's own component, so a
    /// workspace member's `start` script runs where the manifest that declares
    /// it lives — which is the same rule `Package` uses to read that manifest,
    /// applied to the directory rather than to the file.
    ///
    /// # Errors
    ///
    /// [`SmokeRefused::NotAServeProbe`] if the probe does not start a service,
    /// and [`SmokeRefused::NotAdmitted`] if `enforcement` holds no admitted
    /// command for the probe's check. Both are decided here, once, rather than
    /// re-asked on every run: a smoke check that could be built and then find it
    /// had nothing to start would have to answer with a verdict about a state
    /// the caller could have been told about at construction.
    pub fn of(
        root: &Path,
        probe: &RuntimeProbe,
        enforcement: &'a Enforcement,
        limits: Limits,
    ) -> Result<Self, SmokeRefused> {
        if probe.kind() != ProbeKind::Serve {
            return Err(SmokeRefused::NotAServeProbe { kind: probe.kind() });
        }
        let check = probe.proposal().id();
        let admitted = enforcement
            .admitted()
            .find(|command| command.command().check().id() == check)
            .ok_or_else(|| SmokeRefused::NotAdmitted {
                check: check.clone(),
            })?;
        Ok(Self {
            directory: root.join(probe.component()),
            admitted,
            // Read from the check plan rather than taken as a parameter, so that
            // the fingerprint on every result this module produces is the one
            // the decision was made under. A fingerprint that arrived separately
            // is one that could describe a project state the command was not
            // admitted for.
            fingerprint: enforcement.check_plan().fingerprint.clone(),
            limits,
        })
    }

    /// The directory the service will be started in.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.directory
    }

    /// The bounds this check will be held to.
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Start it, watch it, ask it, stop it, and say what came of it.
    ///
    /// Infallible, and that is `probe.rs`'s rule for the same reason: every way
    /// this can go wrong is a state the check has to *report*, and a caller that
    /// had to handle an `Err` here would be a caller deciding what to tell
    /// somebody about a check that did not happen. A service that cannot be
    /// started is an `error` result with the runner's own words in it, not a
    /// failure of this function.
    #[must_use]
    pub fn run(&self) -> CheckResult {
        let check = self.admitted.command().check();
        let (status, reason) = self.observe();
        let id = check.id().clone();
        let title = check.title().to_owned();
        let severity = check.severity();
        let critical = check.critical();
        let fingerprint = self.fingerprint.clone();
        let class = EvidenceClass::ObservedFact;
        match status {
            CheckStatus::Pass => {
                CheckResult::pass(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            CheckStatus::Fail => {
                CheckResult::fail(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            CheckStatus::Warning => {
                CheckResult::warning(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            CheckStatus::Unknown => {
                CheckResult::unknown(id, title, severity, critical, class, fingerprint)
                    .with_reason(reason)
            }
            // `Error` is what a start that did not happen is worth, and the
            // evidence class is `Unknown` there because nothing was observed —
            // `errored` is the constructor that says so, and it is the reason
            // the two arms below do not take a class.
            //
            // `Skipped` cannot occur: nothing here returns it, and the checks
            // that do are the mode's, decided in `enforce.rs` before a
            // `StartSmoke` can exist. It is named rather than swallowed by `_`
            // so that adding a variant to `CheckStatus` is a compile error in
            // this file instead of a silent mis-mapping, and it is sent to
            // `errored` because that is the arm that refuses to let anything
            // through as a pass.
            CheckStatus::Error | CheckStatus::Skipped => {
                CheckResult::errored(id, title, severity, critical, reason, fingerprint)
            }
        }
    }

    /// Run it and return the status and the sentence together.
    ///
    /// **One function rather than two**, which is where this differs from
    /// `probe.rs` and `browser.rs`: both of those expose `status()` and
    /// `reason()` separately and argue that the two run the same rows in the
    /// same order. Here the rows are the same rows and they are walked once, so
    /// there is no order for the two to disagree about.
    fn observe(&self) -> (CheckStatus, String) {
        let supervisor = Supervisor::new(&self.directory, self.limits.service);
        let service = match supervisor.start(self.admitted) {
            Ok(service) => service,
            Err(error) => {
                return (
                    CheckStatus::Error,
                    format!("SURE could not start it, so nothing about it was checked: {error}"),
                );
            }
        };

        let outlived_the_window = !self.wait_out(&service);
        let asked = if self.limits.endpoint.is_some() && outlived_the_window {
            self.ask()
        } else {
            None
        };

        // The stop is called on every path, including the one where the service
        // has already ended: it is what returns the outcome, and an outcome
        // carries the exit code, both streams and how long the run took. A
        // `Service` that is simply dropped would be stopped too — that is its
        // documented default — and nothing would be reported.
        let outcome = match service.stop() {
            Ok(outcome) => outcome,
            Err(error) => {
                return (
                    CheckStatus::Error,
                    format!("SURE started it and cannot say what it did: {error}"),
                );
            }
        };

        let mut clauses: Vec<String> = Vec::new();
        let status = match outcome.termination() {
            Termination::Exited { code } => {
                clauses.push(if outlived_the_window {
                    format!(
                        "the service was still running after {}, and had ended by itself by the \
                         time SURE stopped it ({})",
                        spoken(self.limits.window),
                        ended_with(code)
                    )
                } else {
                    format!(
                        "the service ended by itself after {}, before SURE could ask it anything \
                         ({})",
                        spoken(outcome.took()),
                        ended_with(code)
                    )
                });
                CheckStatus::Fail
            }
            Termination::Cancelled { .. } => {
                clauses.push(format!(
                    "the service was still running after {}, so SURE stopped it",
                    spoken(self.limits.window)
                ));
                asked
                    .as_ref()
                    .map_or(CheckStatus::Warning, |(_, answer)| answer.status())
            }
            Termination::TimedOut { .. } => {
                clauses.push(format!(
                    "the service ran until its own budget of {} ran out, and SURE stopped it",
                    spoken(self.limits.service.timeout())
                ));
                asked
                    .as_ref()
                    .map_or(CheckStatus::Warning, |(_, answer)| answer.status())
            }
            // Unreachable: `Supervisor::start` returns `Ok` only once the run is
            // under way, so a start that was refused before it began comes back
            // as an `Err` and never as this outcome. It is written out rather
            // than folded into an arm above because the alternative is to report
            // a termination nobody observed as though somebody had, and there is
            // no status it could honestly carry.
            Termination::CancelledBeforeStart => {
                return (
                    CheckStatus::Error,
                    format!(
                        "SURE cannot say what this service did: a start that returned reported \
                         {:?}",
                        outcome.termination()
                    ),
                );
            }
        };

        if let Some((endpoint, answer)) = &asked {
            clauses.push(format!(
                "SURE asked it {}: {}",
                endpoint.request_line(),
                answer.reason()
            ));
        } else if outlived_the_window {
            // Reached exactly when the service was still running and no address
            // was given, which is the one state the warning is about: `asked` is
            // `Some` whenever there was an endpoint *and* the window closed, so
            // the absence of a question here is the absence of somewhere to ask.
            clauses.push(
                "there was no address to ask it at, so SURE has no evidence that the feature \
                 works"
                    .to_owned(),
            );
        }

        if let Some((stream, quoted, dropped)) = last_words(&outcome) {
            clauses.push(format!("it wrote on {stream}: {quoted}"));
            if dropped > 0 {
                clauses.push(format!("and {dropped} earlier lines were not kept"));
            }
        }

        (status, clauses.join("; "))
    }

    /// Whether the service ended before the window closed.
    ///
    /// `true` means it ended, `false` means the window closed with it still
    /// running. The check comes before the first sleep, so a service that was
    /// already gone when the start returned — which the operating system can
    /// report either way round — is not given [`POLL`] of grace.
    fn wait_out(&self, service: &Service) -> bool {
        let deadline = Instant::now() + self.limits.window;
        loop {
            if service.has_finished() {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            std::thread::sleep(POLL.min(deadline - now));
        }
    }

    /// Ask the service the one question, and report what came back.
    ///
    /// Called only while the service is still running — see the module comment
    /// for why that is a rule and not an ordering. The endpoint is always there
    /// when this is called; the `Option` is unwrapped with a `match` rather than
    /// an `expect` because shipped code in this workspace has no `expect`.
    fn ask(&self) -> Option<(Endpoint, ProbeOutcome)> {
        let endpoint = self.limits.endpoint.as_ref()?.clone();
        let answer = Probe::new(self.limits.request).get(&endpoint);
        Some((endpoint, answer))
    }
}

/// How a program's own exit reads in a sentence.
fn ended_with(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code, because the operating system ended it".to_owned(),
    }
}

/// A duration in the words a report prints.
///
/// `Duration`'s own `Debug` — `5s`, `250ms` — is what every test message in this
/// repository uses and is the wrong form for a sentence a person reads: it is a
/// unit symbol with no space in it. Milliseconds below a second and seconds
/// above, which covers every budget a smoke check is given.
fn spoken(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        return format!("{millis} milliseconds");
    }
    if duration.subsec_nanos() == 0 {
        let whole = duration.as_secs();
        return if whole == 1 {
            "1 second".to_owned()
        } else {
            format!("{whole} seconds")
        };
    }
    format!("{:.1} seconds", duration.as_secs_f64())
}

/// The last few lines a service wrote, and how many lines were left out.
///
/// Standard error first and standard output only when that is empty. A program
/// that failed writes its failure to the first, and quoting both would put a
/// build's entire log into one check's reason line — which is the shape of
/// unusable report this product is against.
///
/// The bytes SURE kept are a **beginning** when [`CapturedOutput::was_truncated`]
/// says so, so what is quoted here can be the first five lines of a stream
/// rather than its last five. That is stated rather than hidden: the count of
/// bytes not kept is in the sentence, and a reader who needs the tail of a
/// stream this large is being told that SURE does not have it.
///
/// **The service's own words are escaped before they are placed in a sentence
/// SURE writes**, which is [`crate::setup`]'s `in_a_sentence` treatment applied
/// to the one input in this workspace that is not a name a project chose but
/// whatever a project's process decided to write. `\n` cannot reach here — it is
/// what the lines were split on — but every other control character can, because
/// [`CapturedOutput::text_lossy`] replaces only what is not valid UTF-8: a lone
/// `\r` returns a terminal's cursor to column 0, and an escape sequence such as
/// `\x1b[2K` erases the line it is printed on, so a service that writes them can
/// make SURE's report of the service's own failure **unsay itself**. That is a
/// false green in the terminal rather than in the verdict, and no function of
/// the quote's is worth it. The escaping happens **before** the character bound
/// rather than after it, so that [`QUOTED_CHARS`] keeps bounding what a reader
/// actually sees: an escape sequence the service wrote is six characters of
/// SURE's report, and a bound applied to the raw bytes would let a service hold
/// a reason line open six times longer than the constant says it can be.
fn last_words(outcome: &Outcome) -> Option<(&'static str, String, usize)> {
    for (stream, captured) in [
        ("standard error", outcome.stderr()),
        ("standard output", outcome.stdout()),
    ] {
        let text = captured.text_lossy();
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let dropped = lines.len().saturating_sub(QUOTED_LINES);
        let kept = &lines[dropped..];
        if kept.is_empty() {
            continue;
        }
        let mut quoted = crate::redact::escape_control_characters(&kept.join(" / "));
        if quoted.chars().count() > QUOTED_CHARS {
            quoted = quoted.chars().take(QUOTED_CHARS).collect::<String>();
            quoted.push('…');
        }
        if captured.was_truncated() {
            quoted.push_str(&format!(
                " [{}, and SURE did not keep all of this stream]",
                captured.discarded_bytes()
            ));
        }
        return Some((stream, quoted, dropped));
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::process::CapturedOutput;
    use std::ffi::OsString;

    /// An outcome built by hand, which is the only place this module's quoting
    /// can be tested without a process: every branch of it is about text that
    /// arrived, and a real process writes whatever it likes.
    fn an_outcome(stdout: &str, stderr: &str) -> Outcome {
        Outcome::new(
            OsString::from("python"),
            Termination::Exited { code: Some(1) },
            CapturedOutput::new(stdout.as_bytes().to_vec(), 0, None),
            CapturedOutput::new(stderr.as_bytes().to_vec(), 0, None),
            std::time::SystemTime::now(),
            Duration::from_millis(5),
        )
    }

    #[test]
    fn a_duration_reads_as_a_person_says_it() {
        assert_eq!(spoken(Duration::from_millis(250)), "250 milliseconds");
        assert_eq!(spoken(Duration::from_millis(999)), "999 milliseconds");
        assert_eq!(spoken(Duration::from_secs(1)), "1 second");
        assert_eq!(spoken(Duration::from_secs(5)), "5 seconds");
        assert_eq!(spoken(Duration::from_millis(1500)), "1.5 seconds");
        assert_eq!(spoken(Duration::from_millis(7250)), "7.2 seconds");
    }

    #[test]
    fn a_zero_window_is_refused_rather_than_rounded() {
        assert_eq!(
            Limits::new(
                Duration::ZERO,
                process::Limits::new(Duration::from_secs(60), 1024, 1024),
                probe::Limits::new(Duration::from_secs(5), 1024),
                None,
            )
            .unwrap_err(),
            LimitsError::ZeroWindow
        );
        assert!(!LimitsError::ZeroWindow.to_string().is_empty());
    }

    #[test]
    fn a_window_that_cannot_close_before_the_budget_does_is_refused() {
        // The boundary is inclusive, and that is the point rather than a
        // detail: the budget is measured from the start and the window from the
        // moment the start returned, so an exactly-equal pair is already a
        // window that cannot close first.
        for window in [Duration::from_secs(60), Duration::from_secs(61)] {
            assert_eq!(
                Limits::new(
                    window,
                    process::Limits::new(Duration::from_secs(60), 1024, 1024),
                    probe::Limits::new(Duration::from_secs(5), 1024),
                    None,
                )
                .unwrap_err(),
                LimitsError::WindowOutlastsTheBudget,
                "{window:?}"
            );
        }
        assert!(
            Limits::new(
                Duration::from_millis(59_999),
                process::Limits::new(Duration::from_secs(60), 1024, 1024),
                probe::Limits::new(Duration::from_secs(5), 1024),
                None,
            )
            .is_ok()
        );
        assert!(!LimitsError::WindowOutlastsTheBudget.to_string().is_empty());
    }

    #[test]
    fn a_limits_hands_back_what_it_was_given() {
        let endpoint = Endpoint::loopback(5173, "/").unwrap();
        let service = process::Limits::new(Duration::from_secs(30), 4096, 4096);
        let request = probe::Limits::new(Duration::from_secs(5), 8192);
        let limits = Limits::new(
            Duration::from_secs(2),
            service,
            request,
            Some(endpoint.clone()),
        )
        .unwrap();
        assert_eq!(limits.window(), Duration::from_secs(2));
        assert_eq!(limits.service(), service);
        assert_eq!(limits.request(), request);
        assert_eq!(limits.endpoint(), Some(&endpoint));
    }

    #[test]
    fn the_quote_is_the_tail_and_standard_error_comes_first() {
        let outcome = an_outcome(
            "the banner\nmore noise\n",
            "starting\nlistening on 5173\nboom\n",
        );
        let (stream, quoted, dropped) = last_words(&outcome).unwrap();
        assert_eq!(stream, "standard error");
        assert_eq!(quoted, "starting / listening on 5173 / boom");
        assert_eq!(dropped, 0);

        // With nothing on standard error, standard output is what there is —
        // and it is quoted as text rather than passed over.
        let outcome = an_outcome("the only thing said\n", "");
        let (stream, quoted, dropped) = last_words(&outcome).unwrap();
        assert_eq!(stream, "standard output");
        assert_eq!(quoted, "the only thing said");
        assert_eq!(dropped, 0);

        // A service that said nothing at all has nothing to quote, which is not
        // an empty sentence in the reason.
        assert!(last_words(&an_outcome("", "   \n\n")).is_none());
    }

    #[test]
    fn a_service_cannot_write_an_escape_into_the_line_sure_prints() {
        // The service is a project's own process, so its output is whatever the
        // project decided to write. `\n` cannot survive — it is what the lines
        // were split on — but the rest of the control characters can, and two of
        // them are worth naming: `\r` sends a terminal's carriage back to column
        // 0, and `\x1b[2K` erases the line it is printed on. Quoted unescaped
        // they let a failing service erase SURE's report of the failure as the
        // reader is looking at it, which is a false green in the terminal rather
        // than in the verdict.
        let hostile = "ok\x1b[2K\rSURE: 0 problems, all checks passed\u{7}";
        let outcome = an_outcome("", hostile);
        let (_, quoted, _) = last_words(&outcome).unwrap();
        assert!(
            !quoted.chars().any(char::is_control),
            "a control character survived into the line SURE prints: {quoted:?}"
        );

        // The service's words are still quoted — escaping is the fix, not a
        // reason to stop saying what it wrote — and each control character is
        // shown as the escape it is, so a reader can see that the service wrote
        // it rather than SURE.
        assert!(
            quoted.contains("SURE: 0 problems, all checks passed"),
            "the service's words are not quoted at all: {quoted}"
        );
        assert!(
            quoted.contains(r"\u{001b}"),
            "the escape is not shown: {quoted}"
        );
        assert!(
            quoted.contains(r"\r"),
            "the carriage return is not shown: {quoted}"
        );
        assert!(
            quoted.contains(r"\u{0007}"),
            "the bell is not shown: {quoted}"
        );
    }

    #[test]
    fn more_lines_than_the_quote_keeps_are_counted_rather_than_dropped_quietly() {
        let long: String = (1..=10).map(|n| format!("line {n}\n")).collect();
        let outcome = an_outcome("", &long);
        let (_, quoted, dropped) = last_words(&outcome).unwrap();
        assert_eq!(dropped, 10 - QUOTED_LINES);
        assert!(
            quoted.starts_with(&format!("line {}", dropped + 1)),
            "the tail rather than the head: {quoted}"
        );
        assert!(quoted.contains("line 10"), "{quoted}");
    }

    #[test]
    fn one_enormous_line_is_cut_to_the_character_bound_rather_than_printed_whole() {
        // A minified build error is one line and the line bound does not touch
        // it, which is why the quote is bounded twice.
        let huge = "x".repeat(QUOTED_CHARS * 3);
        let outcome = an_outcome("", &huge);
        let (_, quoted, _) = last_words(&outcome).unwrap();
        assert_eq!(
            quoted.chars().count(),
            QUOTED_CHARS + 1,
            "the ellipsis is one character"
        );
        assert!(quoted.ends_with('…'), "{quoted}");
    }

    #[test]
    fn the_character_bound_counts_characters_and_not_bytes() {
        // Cutting a `String` at a byte index would panic inside shipped code on
        // any project that prints a non-ASCII character, and this machine's own
        // locale is one where that is the first thing a message contains.
        let huge = "中".repeat(QUOTED_CHARS * 2);
        let outcome = an_outcome("", &huge);
        let (_, quoted, _) = last_words(&outcome).unwrap();
        assert_eq!(quoted.chars().count(), QUOTED_CHARS + 1);

        // **Under the bound in characters and over it in bytes**, which is the
        // half the case above cannot see: at twice the bound *both* counts are
        // over it, so a bound measured in bytes cuts to the same place by
        // accident and answers the assertion above identically. Here the line
        // must come through whole. A bound counted in bytes would cut a line
        // SURE kept all of and put an ellipsis on it, which is a report claiming
        // there was more to a line that had all of itself in it.
        let under = "中".repeat(QUOTED_CHARS / 2);
        assert!(
            under.len() > QUOTED_CHARS,
            "this case is only about the difference while the two counts \
             disagree: {} bytes against a bound of {QUOTED_CHARS}",
            under.len()
        );
        let outcome = an_outcome("", &under);
        let (_, quoted, _) = last_words(&outcome).unwrap();
        assert_eq!(
            quoted.chars().count(),
            QUOTED_CHARS / 2,
            "a line under the bound was cut anyway"
        );
        assert!(
            !quoted.ends_with('…'),
            "and it was told there was more: {quoted}"
        );
        assert_eq!(quoted, under, "the quote is not the line it was given");
    }

    #[test]
    fn a_stream_sure_did_not_keep_whole_says_so_in_the_quote() {
        let outcome = Outcome::new(
            OsString::from("python"),
            Termination::Exited { code: None },
            CapturedOutput::empty(),
            CapturedOutput::new(b"the last thing it said\n".to_vec(), 40_000, None),
            std::time::SystemTime::now(),
            Duration::from_millis(1),
        );
        let (_, quoted, _) = last_words(&outcome).unwrap();
        assert!(
            quoted.contains("40000"),
            "the count of bytes not kept belongs in the sentence: {quoted}"
        );
    }

    #[test]
    fn an_exit_with_no_code_says_which_kind_of_ending_that_was() {
        assert_eq!(ended_with(Some(0)), "exit code 0");
        assert_eq!(ended_with(Some(130)), "exit code 130");
        assert!(
            ended_with(None).starts_with("no exit code"),
            "a fabricated code would be read as the program's own"
        );
    }
}

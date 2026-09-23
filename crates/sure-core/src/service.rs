//! Starting something up, keeping it under control, and stopping it.
//!
//! A *service* here is a command from the plan that is meant to keep running
//! while something else is done to it — a server a later check talks to, a
//! watcher a later check waits on. [`Supervisor`] starts one and hands back a
//! [`Service`], which is the handle that keeps it from outliving its welcome.
//!
//! # Policy, not mechanism
//!
//! This module decides *when* something may run and what the caller is told; the
//! deciding itself is not here. It is the same split as [`crate::consent`] and
//! [`crate::enforce`] against [`crate::process`], and it is why the entry point
//! takes an [`AdmittedCommand`]:
//!
//! ```text
//! ExecutionPermissions -> PermissionPlan -> Enforcement::admitted() -> AdmittedCommand
//!                                                                           |
//!                                                                    Supervisor::start
//!                                                                           |
//!                                                               process::run (on a thread)
//! ```
//!
//! [`Supervisor::start`] takes **nothing else**. Not a program name, not an
//! argument list, not a `&str`. A caller cannot hand this module a command line,
//! so a service cannot be started that [`Enforcement::admitted`] did not admit —
//! and because [`AdmittedCommand`] has no public constructor, that is a fact
//! about a type rather than a rule for callers to follow. What the service *is*,
//! as evidence, is the [`PlannedCommand`] it was started from: it carries the
//! check, the arguments, the mode, and the decision that let it run.
//!
//! # The knobs, and what they mean
//!
//! [`Supervisor::new`] takes a working directory and a [`Limits`], and
//! [`Supervisor::with_environment`] says what the service's environment is.
//! Three things about that are worth stating rather than leaving to be
//! discovered:
//!
//! **`Limits::timeout` is the whole-life budget of the service, not a startup
//! timeout.** A service still running when the budget expires is stopped and
//! comes back [`Termination::TimedOut`](crate::process::Termination::TimedOut),
//! whether it never came up or came up and was working. Nothing here measures
//! how long a start takes, and a caller that wanted a startup deadline would be
//! asking for something this build does not have.
//!
//! **Readiness is not decided here.** [`Supervisor::start`] returning `Ok` means
//! the operating system accepted the spawn and SURE is reading the process — and
//! nothing more. Whether anything is *listening*, whether it answers, and how
//! long that took, is a question for a probe, which is `P3-T010`. A service that
//! starts and immediately dies is `start` returning `Ok` followed by
//! [`Service::has_finished`] being true. Reporting readiness from the fact that a
//! process exists is the false green this product is built against, so the fact
//! is named at the type level instead: this module has no `is_ready`, and the
//! word does not appear in its vocabulary.
//!
//! **The environment is a choice, and a supervisor that was not told one has
//! chosen inheritance.** [`Supervisor::new`] sets [`Environment::inherited`],
//! which is the default [`ProcessRequest::new`] gives every request: a service
//! can find the tools it needs by name, which is what a real service does, and it
//! is also what makes this the wrong shape for confining one — a variable in
//! SURE's own environment is passed to the project.
//! [`Supervisor::with_environment`] is how a caller states something else, and
//! [`crate::runtime_start`] is a caller that does: **a service check is started
//! with the environment the plan holds**, so what the project's process starts
//! with is what the user consented to rather than what SURE happened to carry.
//! Both directions are tested, in this file, against
//! [`Supervisor::request`] — the request a start is about to be built from —
//! because "the environment is stated" is a claim that has to be readable
//! somewhere other than the call site that states it. Neither setting is a
//! sandbox; [`Environment`]'s own documentation says why, and this module says
//! the same thing below about what is missing.
//!
//! # What a caller has to do, and what happens if it does not
//!
//! A [`Service`] **stops itself when it is dropped**, and dropping it does not
//! wait for the outcome — so the process tree is stopped and the handle that
//! would have reported what came of it is gone. That is the right default for
//! something that by definition does not end on its own, and it is the safe
//! direction: a forgotten service is stopped rather than left running.
//! [`Service::stop`] is what a caller calls to stop one *and* be told what it
//! printed.
//!
//! # Logs arrive when the run ends
//!
//! [`Service::stop`] returns an [`Outcome`] carrying both captured streams,
//! bounded by [`Limits::stdout_bytes`] and [`Limits::stderr_bytes`] — and each
//! arrives **once**, at the end, not as it is written. That is not a choice made
//! here: [`crate::process`] reads each stream to end-of-file on its own thread
//! and reports it when the pipe closes, and a stream handed out while it is still
//! being written is a beginning a reader cannot tell from the whole. A caller
//! that needs a service's output while the service is still up is asking for
//! something this build does not have, and the honest answer is that it is
//! missing rather than that it is half there.
//!
//! # What is missing
//!
//! **Nothing confines the service.** The environment is a decision now rather
//! than an accident of a default nobody read — [`Supervisor::with_environment`],
//! and on the check path the plan is what states it — but an environment is a
//! list of strings a program starts with and not a boundary around it. A service
//! can still open any file the user can open, reach the network, and read this
//! machine's other state. [`Environment`]'s own documentation says the same about
//! both of its variants, and it is repeated here because "the environment is
//! controlled" is the kind of sentence a reader turns into "the process is
//! confined".
//!
//! **No restart, and nothing watching the watcher.** A service that exits is not
//! brought back. [`Service::has_finished`] is how a caller finds out, and
//! nothing here calls it on the caller's behalf.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::SystemTime;

use crate::consent::PlannedCommand;
use crate::enforce::AdmittedCommand;
use crate::process::{Cancellation, Environment, Limits, Outcome, ProcessError, ProcessRequest};

/// Starts services and stops them.
///
/// One supervisor describes one working directory, one set of limits and one
/// environment, and can start any number of services under them. A start changes
/// nothing it holds, so it is shared freely; it is not `Clone` because it is not
/// a value to copy around, it is a thing to keep.
#[derive(Debug)]
pub struct Supervisor {
    working_directory: PathBuf,
    limits: Limits,
    environment: Environment,
}

impl Supervisor {
    /// A supervisor that starts services in `working_directory`, held to
    /// `limits` and given [`Environment::inherited`] unless a caller says
    /// otherwise with [`Supervisor::with_environment`].
    ///
    /// The directory is not checked here and is not checked until a start is
    /// attempted, for the reason [`ProcessRequest::new`] gives: a request
    /// describes something that is going to happen, and a directory can be
    /// removed between the describing and the doing. So a supervisor for a
    /// directory that is not there is constructible, and [`Supervisor::start`]
    /// is where that is found out — at the start, before anything runs, rather
    /// than as a run that quietly did nothing.
    ///
    /// The environment is set here rather than left to the request's own default
    /// so that the choice is made once, in this module, where the module comment
    /// argues it: a supervisor that was handed no environment inherits SURE's,
    /// and that is a decision this constructor makes rather than a line a reader
    /// has to find in another file.
    #[must_use]
    pub fn new(working_directory: impl Into<PathBuf>, limits: Limits) -> Self {
        Self {
            working_directory: working_directory.into(),
            limits,
            environment: Environment::inherited(),
        }
    }

    /// The directory services will be started in.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// The bounds every service this supervisor starts is held to.
    #[must_use]
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// State what the service's environment is, in place of SURE's own.
    ///
    /// Takes `self` and returns it, the shape [`ProcessRequest::with_environment`]
    /// has, because a supervisor is one value describing one way of starting
    /// things rather than a builder a caller keeps a half-finished copy of.
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// The environment every service this supervisor starts is given.
    #[must_use]
    pub const fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The request `command` would be started with, without starting it.
    ///
    /// **[`Supervisor::start`] builds its request here and nowhere else**, so
    /// this is the same value a run gets — not a description of one — and a
    /// caller can read it, and a test can compare it against the command it came
    /// from, without a process existing. It is the same shape
    /// `planned_check_runner.rs`'s `AdmittedRun::request` has, for the same
    /// reason: the translation from a decision to a request is the part most
    /// likely to be got subtly wrong, and the only way to measure it is to be
    /// able to hold it.
    ///
    /// The program, the arguments and the directory are the admitted command's
    /// own values, handed over unchanged, and `cancellation` is the only thing
    /// added here.
    #[must_use]
    pub fn request(
        &self,
        command: AdmittedCommand<'_>,
        cancellation: Cancellation,
    ) -> ProcessRequest {
        ProcessRequest::new(
            command.command().program(),
            &self.working_directory,
            self.limits,
            cancellation,
        )
        .with_arguments(command.command().arguments().iter().cloned())
        .with_environment(self.environment.clone())
    }

    /// Start `command` as a service.
    ///
    /// Returns once the process is under way, and **only** once: every path that
    /// returns an error here is a path on which nothing was started, with the
    /// one exception [`ServiceError::NotFollowed`] names. The distinction is not
    /// cosmetic — a caller told "it failed" will go on to do something else, and
    /// a process loose at that point is a process nobody holds a handle to.
    ///
    /// # Errors
    ///
    /// * [`ServiceError::Runner`], carrying a [`ProcessError`] — a working
    ///   directory that is not a full path or is not usable, or a program the
    ///   operating system would not start. Nothing ran on any of these paths.
    /// * [`ServiceError::NotFollowed`] when SURE cannot say what the run did.
    ///   **This is the one error that is not a statement that nothing ran**, and
    ///   it says so.
    pub fn start(
        &self,
        command: AdmittedCommand<'_>,
        cancellation: Cancellation,
    ) -> Result<Service, ServiceError> {
        // The same call a caller can make, and that is the point: the request a
        // start uses is not built by a second expression that could drift from
        // the one [`Supervisor::request`] hands back to a test. `AdmittedCommand`
        // is `Copy`, so the value is still here to be read below.
        let request = self.request(command, cancellation.clone());
        let command = command.command().clone();

        let (under_way, started) = mpsc::channel::<()>();
        // The receiver is used once, below, and the sender is moved into the run
        // thread and dropped with it. A `send` that fails therefore means the
        // receiver is gone, which happens only if this function returned before
        // `recv` — and it cannot: `recv` is the next thing that happens.
        let running = std::thread::spawn(move || {
            crate::process::run_when_started(&request, move || {
                // A send on an unbounded channel does not block and does not
                // allocate. Both matter: the run is waiting on this returning,
                // and `process`'s own documentation says what a slow callback
                // costs.
                let _ = under_way.send(());
            })
        });

        match started.recv() {
            Ok(()) => Ok(Service {
                command,
                working_directory: self.working_directory.clone(),
                started_at: SystemTime::now(),
                stopper: Stopper(cancellation),
                running,
            }),
            // The sender was dropped without sending, and it is dropped when the
            // run's closure is dropped — so on this arm the run is over, and
            // every path that reaches it without being under way is one that
            // returned before the spawn. `awaited` says which.
            Err(_) => match awaited(running) {
                Err(error) => Err(error),
                // A run that got under way sends before it waits for anything,
                // so this arm is unreachable while `run_when_started` keeps the
                // contract its own documentation states. It is written out
                // rather than folded into the arm above because the alternative
                // is to throw away an outcome — and a run that ended and cannot
                // be vouched for is exactly what `NotFollowed` is for, whichever
                // way it got there. **No test reaches this**: the way to make it
                // reachable is to break the contract in the other module, and a
                // test cannot break it.
                Ok(outcome) => Err(ServiceError::NotFollowed {
                    message: format!(
                        "the run ended as {:?} without ever reporting that it was under way",
                        outcome.termination()
                    ),
                }),
            },
        }
    }
}

/// A service that is running, or has ended and has not been read yet.
///
/// Dropping one stops it. See the module documentation for what that does and
/// does not tell you.
#[derive(Debug)]
pub struct Service {
    command: PlannedCommand,
    working_directory: PathBuf,
    started_at: SystemTime,
    stopper: Stopper,
    running: JoinHandle<Result<Outcome, ProcessError>>,
}

impl Service {
    /// The command this service was started from, with the decision that let it.
    #[must_use]
    pub fn command(&self) -> &PlannedCommand {
        &self.command
    }

    /// The directory the service was started in.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// When SURE saw the process start.
    ///
    /// Held here rather than read out of the [`Outcome`], because the outcome is
    /// not available until the service has ended and "when did this start" is
    /// the question a caller waiting on a service asks while it is still up.
    #[must_use]
    pub const fn started_at(&self) -> SystemTime {
        self.started_at
    }

    /// Whether the run has ended — by itself, by its timeout, or by a stop.
    ///
    /// **True does not mean the service came up and then succeeded.** It means
    /// the run is over and [`Service::stop`] can be called to find out how. A
    /// service that died on its first line and one that served for an hour both
    /// answer `true` here, and the outcome is what tells them apart.
    #[must_use]
    pub fn has_finished(&self) -> bool {
        self.running.is_finished()
    }

    /// Stop the service and report what came of it.
    ///
    /// The stop asks for the process tree and not the process — a server started
    /// through a shell, or one that starts workers of its own, is stopped whole
    /// **on Windows, and only there.** That is what `taskkill /T /F` reporting
    /// success means; every other platform runs [`Child::kill`] on the one
    /// process SURE holds and reports
    /// [`Stop::ProcessOnly`](crate::process::Stop::ProcessOnly), so anything the
    /// service started there may still be running. **How much was reached is
    /// reported rather than assumed** — it is the `stopped` field on the
    /// returned [`Outcome`]'s
    /// [`Terminated`](crate::process::Termination::Cancelled) — and the reason
    /// the other platforms stop one process is in [`crate::process`]: a process
    /// group would need `unsafe`, which this workspace forbids. The returned
    /// [`Outcome`] also carries both captured streams and a termination saying
    /// which stop it was:
    /// [`Cancelled`](crate::process::Termination::Cancelled) for this one,
    /// [`TimedOut`](crate::process::Termination::TimedOut) if the budget had
    /// already run out, or
    /// [`Exited`](crate::process::Termination::Exited) if the service ended by
    /// itself before the stop arrived.
    ///
    /// [`Child::kill`]: std::process::Child::kill
    ///
    /// # Errors
    ///
    /// [`ServiceError::Runner`] if the run itself failed — including the case
    /// where the process started and could not be stopped, which is
    /// [`ProcessError::CouldNotBeWatched`] — and [`ServiceError::NotFollowed`]
    /// if it started and could not be followed. A stop is never itself an error:
    /// a process that ran and was stopped is an outcome, because its output is
    /// the evidence for why it was stopped.
    pub fn stop(self) -> Result<Outcome, ServiceError> {
        // Every field is bound, and that is load-bearing rather than tidy: a
        // partially moved value keeps its un-moved fields until the *binding*
        // goes out of scope, which here is the end of this function — so
        // `let Service { running, .. } = self;` would leave `stopper` to be
        // dropped after the wait below, and the wait is on a run that only ends
        // because `stopper` is dropped. Measured with a `Drop` that prints,
        // rather than reasoned about.
        let Service {
            command,
            working_directory,
            started_at,
            stopper,
            running,
        } = self;
        // The cancellation comes first for the same reason: the run is asked to
        // stop before anything waits for it to have stopped.
        drop(stopper);
        drop((command, working_directory, started_at));
        awaited(running)
    }
}

/// What actually asks a run to stop, in a type that is allowed to have a `Drop`.
///
/// Separate from [`Service`] because [`Service::stop`] takes `self` and has to
/// move the [`JoinHandle`] out of it, and a type with a `Drop` cannot have its
/// fields moved out. Putting the `Drop` here leaves `Service` free to be
/// destructured, so the cancellation happens in the order written above rather
/// than in an order nobody chose.
#[derive(Debug)]
struct Stopper(Cancellation);

impl Drop for Stopper {
    fn drop(&mut self) {
        // Idempotent, and deliberately the only thing this does: a service is
        // stopped by the cancellation being set, and whether anyone waits for
        // the outcome afterwards is a different question.
        self.0.cancel();
    }
}

/// Join the run's thread and turn what it did into what the caller sees.
///
/// The `Err` case — `join` failing — is not "the thread panicked in an unknown
/// way": the run thread panics only if something inside it panics, and the only
/// candidates are the reader threads and the wait loop, which are shipped code
/// with no `panic!` in them, no `unwrap`, and no indexing that can go out of
/// range. What is left is a panic nobody has written yet, which is exactly why
/// the payload is carried up in the panic's own words instead of being asserted
/// away: a run whose fate is unknown must be reported as neither a run that
/// succeeded nor a run that never happened.
fn awaited(running: JoinHandle<Result<Outcome, ProcessError>>) -> Result<Outcome, ServiceError> {
    match running.join() {
        Ok(Ok(outcome)) => Ok(outcome),
        Ok(Err(error)) => Err(ServiceError::Runner(error)),
        Err(panicked) => Err(ServiceError::NotFollowed {
            message: panic_message(&panicked),
        }),
    }
}

/// What a panic payload says, when it says anything.
///
/// A payload is an `Any`, and the two things `panic!` puts in one are a `&str`
/// and a `String`. Anything else is a payload somebody built by hand, and saying
/// so is better than saying nothing: an empty message would read as though the
/// reason were unknown when the reason is that the payload was not one of the
/// two shapes a panic has.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        String::from("the run's thread stopped without saying why")
    }
}

/// Why a service could not be started, or why SURE cannot vouch for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// The run failed. Nothing was started, with the one exception
    /// [`ProcessError::CouldNotBeWatched`] carries and explains.
    Runner(ProcessError),
    /// **SURE cannot say what this run did.**
    ///
    /// Two shapes reach it and they are the same position for a caller: the
    /// process started and the run could not be followed to its end, or the run
    /// ended without ever reporting that it was under way — the second of which
    /// `process::run_when_started`'s contract rules out, and which is reported
    /// rather than asserted away because the contract is a paragraph in another
    /// module rather than a fact this one can check.
    ///
    /// It exists as its own variant so that a caller cannot fold it into
    /// [`Self::Runner`] by treating every `Err` as "nothing started". Something
    /// may have started, and this is the variant that says so.
    NotFollowed {
        /// What the run said as it ended, in its own words.
        message: String,
    },
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runner(error) => error.fmt(formatter),
            Self::NotFollowed { message } => write!(
                formatter,
                "SURE cannot say what this service did: {message}"
            ),
        }
    }
}

impl std::error::Error for ServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runner(error) => Some(error),
            Self::NotFollowed { .. } => None,
        }
    }
}

impl From<ProcessError> for ServiceError {
    fn from(error: ProcessError) -> Self {
        Self::Runner(error)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! What a supervisor does without starting anything.
    //!
    //! **Every other test of this module starts a real process** —
    //! `tests/service_supervisor.rs` says so in its own header, and it is right
    //! to. What is left for a unit test here is the one thing that needs no
    //! process at all: the request a start is about to be built from. It is
    //! reachable because [`Supervisor::start`] builds its request through
    //! [`Supervisor::request`], so the value these tests hold is the value a
    //! service would be started with and not a description of one.

    use super::*;
    use std::ffi::{OsStr, OsString};
    use std::time::Duration;

    use sure_domain::execution::{ExecutionMode, ExecutionPermissions};
    use sure_domain::ids::{CheckId, FingerprintId};
    use sure_domain::severity::Severity;

    use crate::consent::{PermissionPlan, PlannedCheck};
    use crate::enforce::Enforcement;

    /// The bounds every fixture here starts a service under.
    fn limits() -> Limits {
        Limits::new(Duration::from_secs(30), 64 * 1024, 64 * 1024)
    }

    /// An enforcement holding one admitted command, built the way the product
    /// builds one: a permission plan holding this program, decided under a mode
    /// that runs the project's code.
    ///
    /// `npm run start` is classified as running code the project controls, which
    /// is what the grant covers, so this command is admitted. The check it is
    /// admitted for is not a plan this file has any opinion about — the request a
    /// start builds does not read the check, only the program, the arguments and
    /// the supervisor's own three knobs. The program is `npm run start` and not a
    /// server module, because a module the classifier does not know needs
    /// permissions this fixture does not grant, and a fixture that admitted nothing
    /// would measure nothing.
    fn an_enforcement(fingerprint: &FingerprintId) -> Enforcement {
        let check = PlannedCheck::new(
            CheckId::parse("chk_service").expect("a well-formed check id"),
            "a service",
            Severity::MustFix,
            true,
        );
        let mut plan = PermissionPlan::new(
            ExecutionMode::HostConfirmed,
            fingerprint.clone(),
            ExecutionPermissions {
                run_project_code: true,
                ..ExecutionPermissions::inspect_only()
            },
        );
        plan.add(check.clone(), "npm", ["run", "start"]);
        Enforcement::of("a fixture", plan, &[check])
    }

    #[test]
    fn a_supervisor_that_was_not_told_inherits_sures_own_environment() {
        let supervisor = Supervisor::new(std::env::temp_dir(), limits());

        assert_eq!(
            supervisor.environment(),
            &Environment::inherited(),
            "a supervisor built with no environment of its own has decided to pass SURE's on, and \
             the decision is readable here rather than assumed by a request's default"
        );
    }

    #[test]
    fn a_supervisor_hands_a_service_the_environment_it_was_given() {
        let fingerprint = FingerprintId::generate();
        let enforcement = an_enforcement(&fingerprint);
        let command = enforcement
            .admitted()
            .next()
            .expect("the fixture's mode admits one command");
        // `Only`, because it is the variant that cannot be confused with the
        // default: a request that quietly inherited would fail this assertion for
        // the reason the whole clause is about.
        let environment = Environment::only([
            (OsString::from("PATH"), OsString::from("/usr/bin")),
            (OsString::from("PORT"), OsString::from("8080")),
        ]);
        let supervisor =
            Supervisor::new(std::env::temp_dir(), limits()).with_environment(environment.clone());

        let request = supervisor.request(command, Cancellation::new());

        assert_eq!(
            request.environment(),
            &environment,
            "the service starts with the environment the supervisor was given, and with none of \
             SURE's own"
        );
        assert!(
            matches!(request.environment(), Environment::Only(_)),
            "and it is the variant itself that arrived, not a copy that was merged with SURE's"
        );
    }

    #[test]
    fn the_request_carries_the_command_and_the_supervisors_own_knobs() {
        let fingerprint = FingerprintId::generate();
        let enforcement = an_enforcement(&fingerprint);
        let command = enforcement
            .admitted()
            .next()
            .expect("the fixture's mode admits one command");
        let directory = std::env::temp_dir().join("sure-service-fixture");
        let supervisor = Supervisor::new(&directory, limits());
        let cancellation = Cancellation::new();

        let request = supervisor.request(command, cancellation.clone());

        assert_eq!(request.program(), OsStr::new("npm"));
        assert_eq!(
            request.arguments(),
            [OsString::from("run"), OsString::from("start")],
            "the argument vector is the command's own, element for element"
        );
        assert_eq!(request.working_directory(), directory);
        assert_eq!(request.limits(), limits());
        cancellation.cancel();
        assert!(
            request.cancellation().is_cancelled(),
            "the handle the caller holds is the one the request will be stopped through, so a \
             caller that stops this service stops the process this request starts"
        );
    }
}

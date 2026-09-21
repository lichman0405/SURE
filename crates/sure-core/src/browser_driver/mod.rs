//! The browser adapter: finding a browser, starting it, and driving it.
//!
//! `P5-T004` acceptance: *"Browser check can navigate supported local app and
//! capture runtime/console errors."* / *"Unavailable browser never reports
//! pass."*
//!
//! # What this is, and what it is not
//!
//! [`crate::browser`] is the interface — the [`BrowserDriver`] trait, the
//! [`Observation`] a driver produces, and the one door from an observation to a
//! verdict. **This module is the adapter that implements it against a real
//! Chrome-speaking browser**, by starting one, connecting to its debugging port
//! and speaking the DevTools protocol over it.
//!
//! It is arranged so that the interface's guarantees survive contact with a real
//! browser. Nothing here returns a
//! [`CheckStatus`](sure_domain::status::CheckStatus) or a
//! [`CheckResult`](sure_domain::status::CheckResult): the whole module produces
//! an [`Observation`] and hands it to [`Report::observed`], and the mapping from
//! there to a verdict is `browser.rs`'s and nobody else's. A reader who wants to
//! know what SURE concludes from a page should read `browser.rs`; a reader who
//! wants to know what SURE saw should read [`session`].
//!
//! # The four steps, and which ones the project can influence
//!
//! 1. **[`installed`]** — find a browser among a table compiled into this binary
//!    and the `PATH`. **The project cannot influence this**, and a relative
//!    `PATH` entry is skipped precisely so that it cannot: SURE is normally run
//!    with the project as its working directory, so `./chrome.exe` is a program
//!    the project could have written.
//! 2. **[`launch`]** — start it headless against a private profile directory,
//!    with the sandbox on. **The project cannot influence the arguments**, and
//!    the page's address is deliberately not among them: it is sent later as a
//!    protocol command, so there is no argument vector for a project-supplied
//!    path to be injected into.
//! 3. **[`session`]** — attach to the page, turn on three domains, and fold
//!    every event into one observation. **The project influences what arrives
//!    here and nothing about how it is read**, and the rules are measured ones —
//!    see that module, which records the three attempts it took to get the
//!    failed-request rule to stop reporting healthy pages as broken.
//! 4. **[`crate::browser::Report::status`]** — a verdict, in the interface.
//!
//! # What the project *does* get to say
//!
//! Exactly one thing: **the address**. It comes from
//! [`Target`](crate::browser::Target), which is built from an address and a
//! path and refuses anything not on this machine — the same constructor the
//! local probe uses, so the loopback rule has one spelling and not two.
//!
//! That is the security posture in one sentence: **the page is untrusted and
//! everything around it is not.** The page is the project's code, it runs, and
//! it is contained by a browser sandbox that is never turned off for
//! convenience. A browser that will not start without `--no-sandbox` is reported
//! as [`AbsenceReason::DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart)
//! and the check is skipped, because a skipped check is a smaller loss than
//! running a project's JavaScript without the boundary that exists to hold it.
//!
//! # Why no new dependency
//!
//! The protocol needs a WebSocket client, and the handshake needs SHA-1, base64,
//! and sixteen bytes of nonce. All four are here rather than in the manifest:
//! [`sha1`], [`base64`] and [`websocket`], each with the argument written above
//! it and each checked against answers somebody else published.
//!
//! **The one that was genuinely considered is randomness**, because
//! `Sec-WebSocket-Key` is specified as nonces and a nonce is a thing you would
//! normally reach for a crate to get. `std::random::random()` is unstable on
//! this toolchain, and `rust-src` is not installed beside it, so what was done
//! instead was to **measure**: [`std::collections::hash_map::RandomState`] was
//! found to produce a fresh key on every call and a disjoint set of keys across
//! processes. That is not a cryptographic source and is not claimed to be —
//! [`websocket`]'s documentation says so in as many words. What makes it enough
//! is what the nonce is for: RFC 6455's randomness guards against an
//! intermediary on the network choosing the connection's bytes, and there is no
//! intermediary between SURE and a browser SURE started on a loopback port.
//!
//! # Unavailable never means pass
//!
//! Every way this module can fail to produce a page ends in
//! [`AbsenceReason`](crate::browser::AbsenceReason), and every absence is
//! `skipped` — never a pass, never a failure of code that never ran. There are
//! three of them here, and the sentences differ because the situations do:
//!
//! | what happened | reason |
//! | --- | --- |
//! | the search found nothing | [`NoDriverInstalled`](crate::browser::AbsenceReason::NoDriverInstalled) |
//! | a browser was found and would not start, or started and would not talk | [`DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart) |
//! | the caller named a program and it is not there | [`DriverWouldNotStart`](crate::browser::AbsenceReason::DriverWouldNotStart) |
//!
//! **A page that fails is not an absence and must never be one.** A page that
//! throws, 404s, or never arrives is an [`Observation`] with problems in it, and
//! the boundary is drawn at the moment a page could first be asked for — not at
//! the moment something goes wrong. Reporting *the tool would not start* for a
//! server that was not listening would answer a question about the machine when
//! the question asked was about the project.
//!
//! # The temporary file this writes, and where
//!
//! A browser started for remote debugging refuses to use a profile that is
//! already the default one, which is what stops a page reaching the debugging
//! port of the browser a person is using. So each run gets a private directory
//! under the system temporary directory, named for the process and a counter,
//! and it is removed when the run ends. Nothing of the user's is opened, read or
//! written, and no run leaves a profile for the next one to inherit.

use std::path::{Path, PathBuf};

use crate::browser::{BrowserDriver, Limits, Report, Target};

mod base64;
mod installed;
mod launch;
mod session;
mod sha1;
mod websocket;

/// The browser SURE would drive on this machine, or `None`.
///
/// **This is a question about the machine and not about any project**, which is
/// why it takes no arguments and reads no configuration: it is
/// [`installed::find`] with a name a caller outside this module can use. A
/// caller that has a report to write needs it to explain a skip — *no browser
/// was found* is a sentence somebody will want to act on — and a caller that has
/// a test to write needs it to tell *nothing was there to drive* apart from *a
/// browser was there and this build could not drive it*, which are different
/// facts and only the second is a defect here.
#[must_use]
pub fn find_installed_browser() -> Option<PathBuf> {
    installed::find()
}

/// The adapter, as the interface's [`BrowserDriver`].
///
/// The driver holds the program to run and nothing else. **It does not hold a
/// browser**: a [`BrowserDriver::observe`] call starts one, drives it, and
/// stops it before returning, because a driver that kept one alive between calls
/// would be a process the user did not ask for outliving the check that wanted
/// it.
#[derive(Debug, Clone)]
pub struct Browser {
    program: Option<PathBuf>,
}

impl Browser {
    /// A driver that looks for a browser on this machine.
    #[must_use]
    pub const fn system() -> Self {
        Self { program: None }
    }

    /// A driver that runs `program` and does not look for anything else.
    ///
    /// # What this is for, and what it is not
    ///
    /// A caller that has a browser in a place the search does not look can name
    /// it. **What it must never be is a way for a project to choose what SURE
    /// executes** — the whole of [`installed`] exists to stop that — and the
    /// protection is that this constructor is in the product's code rather than
    /// in anything a project writes. A caller that reads this path out of a
    /// configuration file has removed that protection, and nothing here can
    /// detect it.
    #[must_use]
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: Some(program.into()),
        }
    }

    /// The program this driver was told to run, if it was told.
    #[must_use]
    pub fn program(&self) -> Option<&Path> {
        self.program.as_deref()
    }
}

impl BrowserDriver for Browser {
    /// Opens `target` and reports what the page did.
    ///
    /// Never panics and never returns a verdict: the result is an
    /// [`Absence`](crate::browser::Absence) when no page could be asked for, and
    /// an [`Observation`](crate::browser::Observation) once one could.
    fn observe(
        &self,
        target: &Target,
        limits: &Limits,
        cancellation: &crate::process::Cancellation,
    ) -> Report {
        session::open(self.program.as_deref(), target, limits, cancellation)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    use sure_domain::status::CheckStatus;

    fn a_target() -> Target {
        Target::local(3000, "/").expect("127.0.0.1:3000/ is a loopback target")
    }

    fn a_budget() -> Limits {
        Limits::new(std::time::Duration::from_secs(5), 10).expect("five seconds is a budget")
    }

    /// **The adapter, driven through the interface, on a machine with no
    /// browser at all** — and this is the acceptance sentence *"unavailable
    /// browser never reports pass"* as a test anybody can run.
    ///
    /// Everything about this is real: the value is a `Box<dyn BrowserDriver>`,
    /// the call goes through the trait, and the driver goes as far as trying to
    /// start a program. It stops at the operating system, which refuses to start
    /// a program that is not there, so **no browser is started and none needs to
    /// be installed** — which is what makes the test safe to run anywhere and
    /// worth more than one that is skipped on the machines where the failure
    /// path matters.
    ///
    /// What it asserts is the mapping in both directions: the report is an
    /// absence and not an observation, the reason is the one a browser that
    /// would not start gets, and the status is `skipped` — **not green, and not
    /// a failure of the project**, which is the pair of mistakes this check is
    /// arranged against.
    #[test]
    fn a_browser_that_would_not_start_is_skipped_and_never_a_pass() {
        let driver: Box<dyn BrowserDriver> = Box::new(Browser::with_program(
            "/definitely/not/a/browser/anywhere/chrome",
        ));
        let report = driver.observe(
            &a_target(),
            &a_budget(),
            &crate::process::Cancellation::default(),
        );

        let crate::browser::Report::Absent(absence) = &report else {
            panic!("a program that does not exist produced an observation: {report:?}");
        };
        assert_eq!(
            absence.reason,
            crate::browser::AbsenceReason::DriverWouldNotStart
        );
        assert!(
            absence.detail.contains("chrome"),
            "the detail does not name the program: {:?}",
            absence.detail
        );

        assert_eq!(report.status(), CheckStatus::Skipped);
        assert!(!report.status().is_green());
        let verdict = report.verdict(
            sure_domain::ids::CheckId::generate(),
            &a_target(),
            sure_domain::severity::Severity::MustFix,
            true,
            sure_domain::ids::FingerprintId::generate(),
        );
        assert!(
            verdict.blocks_green(),
            "a critical browser check that could not run must keep the run out of green"
        );
    }

    /// The other half of the same sentence: **a driver told to run a program
    /// runs that program rather than going to look for another one.** A driver
    /// that quietly fell back to the search would make the absence above say
    /// `NoDriverInstalled` on a machine that has a browser — the wrong sentence
    /// about the wrong thing, and one nobody would think to check.
    #[test]
    fn a_named_program_is_the_one_that_is_tried() {
        let named = Browser::with_program("/definitely/not/a/browser/anywhere/chrome");
        let report = named.observe(
            &a_target(),
            &a_budget(),
            &crate::process::Cancellation::default(),
        );
        let crate::browser::Report::Absent(absence) = &report else {
            panic!("a program that does not exist produced an observation: {report:?}");
        };
        assert_eq!(
            absence.reason,
            crate::browser::AbsenceReason::DriverWouldNotStart,
            "a named program that does not exist must not be reported as one that \
             was never looked for"
        );
    }

    #[test]
    fn a_driver_that_was_told_which_program_runs_it_and_one_that_was_not_looks() {
        assert_eq!(Browser::system().program(), None);
        let named = Browser::with_program(PathBuf::from("/nowhere/at/all/chrome"));
        assert_eq!(
            named.program(),
            Some(Path::new("/nowhere/at/all/chrome")),
            "a driver told which program to run must not go looking for another"
        );
    }

    /// **The machine question is answerable without running anything.** This is
    /// deliberately not `assert!(find_installed_browser().is_some())`: a machine
    /// with no browser is a supported machine — that is the whole of
    /// `NoDriverInstalled` — and a test that required one would fail there. What
    /// is asserted is that the answer is either a file this build would drive or
    /// nothing at all, which is the property a caller relies on when it uses the
    /// answer to explain a skip.
    #[test]
    fn the_browser_this_machine_has_is_one_this_build_drives_or_there_is_none() {
        match find_installed_browser() {
            Some(path) => {
                assert!(path.is_file(), "{} is not a file", path.display());
                assert!(
                    installed::is_one_of_ours(&path),
                    "{} is not a browser this build drives",
                    path.display()
                );
            }
            None => println!("no browser on this machine, so a browser check would be skipped"),
        }
    }
}

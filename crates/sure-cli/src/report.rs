//! What a command hands back, and the two ways of writing it down.
//!
//! # One result, two renderers, no third path
//!
//! [`Report`] is the CLI's result vocabulary. Every command produces exactly one
//! of its variants, and each variant has a [`Report::human`] arm and a
//! [`Report::machine`] arm. The two matches are exhaustive and independent: a
//! new variant does not compile until both are written, so there is no state of
//! the world in which a result is renderable one way and not the other.
//!
//! They are independent on purpose. Deriving the human form from the JSON, or
//! the JSON from the human text, is the tempting shortcut and it is the reason
//! "human and machine output are separated" needs saying at all — one path would
//! then be a formatting of the other, and a field could exist in the form a
//! script reads while being absent from the form a person reads, with no test
//! able to name the difference.
//!
//! # The exit status
//!
//! Nothing else in this program decides one. [`Report::exit_code`] is the whole
//! mapping, and `docs/architecture/CLI.md` lists the same codes. A script that
//! reads only the exit status and a person who reads only the prose are looking
//! at the same decision.
//!
//! # The machine-readable frame
//!
//! One JSON object on one line, always with `sure_version`, `protocol_version`,
//! `command`, `outcome` and `exit_code`. The first two are there so that a bug
//! report containing a captured response says which build produced it — the two
//! things that make a stored answer unreadable are a newer SURE and a newer
//! protocol, and a response that does not name either invites the reader to
//! assume the current ones. `command` and `outcome` are what a reader switches
//! on, and `exit_code` is the same decision the process returned, so that a
//! script reading only the body and a script reading only the status are told
//! the same thing.
//!
//! Everything else a command found goes under `details`, one key, written by
//! that command's own module.
//!
//! This frame is **not** one of the seven documents in
//! `docs/architecture/PROTOCOL.md`. Those are statements about a project or an
//! event, each with a schema in `schemas/`; this is the envelope a command
//! answers in. `docs/architecture/PROTOCOL.md` records the distinction, because
//! a sentence there says "everything SURE writes" and a CLI response is written.

use std::io::{self, Write};

use serde_json::json;

/// The exit statuses SURE is allowed to produce.
///
/// The same table is in `docs/architecture/CLI.md`. Two codes are reserved
/// before anything can return them, because a script written against a future
/// release should not have to be rewritten when that release reuses a number
/// for something else.
pub mod exit {
    /// The command did what it says it does.
    pub const OK: u8 = 0;
    /// The command ran, and the answer is not a clean one.
    ///
    /// `sure doctor` returns this when it finds something wrong with this
    /// installation, and `sure check` will return it once a project can be
    /// checked and findings are found. That is one code for two things on
    /// purpose: what a caller does with it is the same — stop — and the report
    /// says which it was. What must never be merged is this and
    /// [`UNAVAILABLE`]: "the project has problems" and "SURE cannot do that
    /// here" need opposite responses from the person reading them, and one
    /// status for both is how a tool that is broken gets read as a project that
    /// is clean.
    pub const NOT_GREEN: u8 = 1;
    /// The command line was wrong.
    pub const USAGE: u8 = 2;
    /// The command exists, and this build cannot carry it out.
    ///
    /// Distinct from [`FAILED`] because the remedy is: this is not a bug report,
    /// and the answer is a newer build rather than a fixed one.
    pub const UNAVAILABLE: u8 = 3;
    /// SURE declined, and can say why in the user's own terms.
    ///
    /// Reserved. It is what a refusal that is a *decision* rather than a
    /// failure returns — a project configuration asking for authority it cannot
    /// be granted, or a location inside the project. `docs/adr/0011` and
    /// `docs/architecture/CONFIG_AUTHORITY.md` are where those decisions live.
    #[allow(
        dead_code,
        reason = "reserved by docs/architecture/CLI.md; nothing refuses yet"
    )]
    pub const REFUSED: u8 = 4;
    /// The command tried and did not finish.
    pub const FAILED: u8 = 5;
}

/// A command this build recognises and cannot yet carry out.
///
/// Every field is a `&'static str`, for the same reason
/// `sure_core::diagnostics`' messages are: a sentence SURE says about itself is
/// fixed, so no call site can paraphrase it into something weaker, and no
/// project-controlled text can be interpolated into it.
///
/// # Why there is no phase in here
///
/// The obvious third field is "which phase implements this", and it was
/// written and then removed. It is a promise SURE cannot keep — no test can
/// check that a phase lands — and for at least one command there is no phase to
/// name: `sure history` is in the documented surface
/// (`docs/architecture/STORAGE_AND_DATA_PATHS.md`) and no task in
/// `tasks/tasks.json` owns it. A field whose value is sometimes invented is
/// worse than a field that is absent, because a reader cannot tell the two
/// apart. Where a command is scheduled is `tasks/tasks.json`'s business, and
/// `docs/architecture/CLI.md` says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotYet {
    /// The command, as the user typed it, taken from the grammar rather than
    /// retyped — see `Command::name`.
    pub command: &'static str,
    /// What the command will do, once this build can. Phrased to follow
    /// "It will …".
    pub does: &'static str,
    /// What SURE did instead. A sentence, ending in a full stop.
    pub instead: &'static str,
}

/// The result of one command.
///
/// Boxed where the payload is large, so that a variant carrying a page of
/// findings does not make every `Report` a page wide. Clippy enforces the
/// general rule; the box here is the answer to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// `sure version`.
    Version,
    /// `sure protocol`.
    Protocol,
    /// `sure doctor`, carrying what it found.
    Doctor(Box<sure_core::doctor::DoctorReport>),
    /// A command whose work lands in a later phase.
    Unavailable(NotYet),
}

impl Report {
    /// The `outcome` value of the machine-readable frame.
    ///
    /// A closed set, and the same strings
    /// `docs/architecture/CLI.md` documents. A reader switches on this;
    /// adding a variant here is adding a case every reader must handle, which
    /// is why it is a short list.
    #[must_use]
    pub const fn outcome(&self) -> &'static str {
        match self {
            Self::Version | Self::Protocol => "ok",
            // A doctor that found something wrong ran perfectly well, and this
            // says what its answer was rather than what the run did. It reads
            // the same predicate as `exit_code`, so the two cannot disagree;
            // they are both here because one is read by a person's script and
            // the other by anything that can read a status.
            Self::Doctor(report) => {
                if report.is_well() {
                    "ok"
                } else {
                    "not_green"
                }
            }
            Self::Unavailable(_) => "unavailable",
        }
    }

    /// The status this program exits with.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Version | Self::Protocol => exit::OK,
            Self::Doctor(report) => {
                if report.is_well() {
                    exit::OK
                } else {
                    exit::NOT_GREEN
                }
            }
            Self::Unavailable(_) => exit::UNAVAILABLE,
        }
    }

    /// Whether the human form is an answer rather than a complaint.
    ///
    /// The one place that decides which stream the human form takes, so that
    /// the decision cannot differ between two call sites. An answer goes to
    /// stdout and a complaint to stderr, which is what makes
    /// `sure history > out.txt` leave the complaint on the terminal.
    ///
    /// `outcome()` is not the same question. A doctor report that found a
    /// problem is still an *answer* — `sure doctor > report.txt` has to put the
    /// report in the file, and the exit status is what carries the bad news. A
    /// complaint is SURE saying it could not do the thing at all.
    #[must_use]
    pub const fn is_an_answer(&self) -> bool {
        match self {
            Self::Version | Self::Protocol | Self::Doctor(_) => true,
            Self::Unavailable(_) => false,
        }
    }

    /// Write the human form.
    ///
    /// Takes the stream rather than reaching for one, so that
    /// [`Format::emit`](crate::output::Format::emit) is the only place in the
    /// crate that names `io::stdout` or `io::stderr`.
    ///
    /// # Errors
    ///
    /// Any failure from `out`.
    pub fn human(&self, out: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Version => writeln!(out, "{}", sure_core::version_string()),
            // What an adapter needs in order to decide whether it can talk to
            // this build, and nothing else. `docs/architecture/PROTOCOL.md`
            // assigns the *handshake* — what an adapter does with the answer —
            // to P1-T010, which owns this command's output from there.
            Self::Protocol => writeln!(
                out,
                "Harness protocol version {}.",
                sure_core::PROTOCOL_VERSION
            ),
            Self::Doctor(report) => crate::doctor::human(report, out),
            Self::Unavailable(not_yet) => {
                let NotYet {
                    command,
                    does,
                    instead,
                } = not_yet;
                writeln!(out, "sure {command} is not implemented in this build.")?;
                writeln!(out)?;
                writeln!(out, "It will {does}. This build stops before that.")?;
                writeln!(out)?;
                writeln!(
                    out,
                    "{instead} SURE exited with status {} rather than {}, because work that was \
                     not done and work that succeeded must never look alike to a script.",
                    exit::UNAVAILABLE,
                    exit::OK
                )
            }
        }
    }

    /// Write the machine-readable form: one object, one line, no newline.
    ///
    /// # Errors
    ///
    /// Any failure from `out`, and any failure to serialize. Serialization
    /// cannot fail for a frame built here out of strings and integers, but it
    /// returns a `Result` all the same, and swallowing one would be the
    /// habit that gets a real failure dropped later.
    pub fn machine(&self, out: &mut impl Write) -> io::Result<()> {
        let frame = self.frame();
        serde_json::to_writer(&mut *out, &frame).map_err(io::Error::other)
    }

    /// The frame [`Report::machine`] writes.
    ///
    /// # Why `exit_code` is always here
    ///
    /// It was first written only for a refusal, on the idea that a field should
    /// appear when it says something. That is the wrong test for this one: it is
    /// one of the two things a caller switches on, and a frame where it is
    /// sometimes `null` is a frame a script has to special-case. The status the
    /// process returned is a fact about every run, so it is in every frame.
    fn frame(&self) -> serde_json::Value {
        let mut frame = json!({
            "sure_version": env!("CARGO_PKG_VERSION"),
            "protocol_version": sure_core::PROTOCOL_VERSION,
            "command": self.command(),
            "outcome": self.outcome(),
            "exit_code": self.exit_code(),
        });
        match self {
            Self::Unavailable(not_yet) => {
                frame["does"] = json!(not_yet.does);
                frame["instead"] = json!(not_yet.instead);
            }
            // Everything a doctor run found, under one key, so that the fields
            // above keep meaning exactly what the module documentation says
            // they mean.
            Self::Doctor(report) => frame["details"] = crate::doctor::machine(report),
            Self::Version | Self::Protocol => {}
        }
        frame
    }

    /// Which command produced this.
    ///
    /// An exhaustive match with no fallback arm, so a new report cannot be
    /// added without naming the command it answers.
    #[must_use]
    pub const fn command(&self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::Protocol => "protocol",
            Self::Doctor(_) => "doctor",
            Self::Unavailable(not_yet) => not_yet.command,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn text(report: &Report, machine: bool) -> String {
        let mut buffer = Vec::new();
        if machine {
            report.machine(&mut buffer).unwrap();
        } else {
            report.human(&mut buffer).unwrap();
        }
        String::from_utf8(buffer).unwrap()
    }

    fn a_refusal() -> NotYet {
        NotYet {
            command: "check",
            does: "check a project and report what it found",
            instead: "Nothing was checked.",
        }
    }

    /// A doctor report carrying the given problems.
    ///
    /// Built rather than produced by `sure_core::doctor::examine`, because what
    /// is under test in this module is the frame, the status and the stream —
    /// not what happens to be in a temporary directory. `is_well` is decided by
    /// `problems` alone, so this is enough to drive both outcomes, and the
    /// end-to-end shape is `tests/cli_contract.rs`'s subject.
    fn a_doctor_report(problems: Vec<sure_core::doctor::Problem>) -> Report {
        use sure_core::doctor::{Build, DoctorReport, Places, StoreState};

        Report::Doctor(Box::new(DoctorReport {
            build: Build {
                version: "0.0.0-test".to_owned(),
                protocol_version: 1,
                os: "test",
                arch: "test",
                running_from: None,
            },
            places: Places::Unknown {
                what: "its evidence and history",
                detail: "not this test's subject".to_owned(),
            },
            store: StoreState::NotLookedFor,
            tools: Vec::new(),
            problems,
            not_checked: Vec::new(),
        }))
    }

    fn a_problem() -> sure_core::doctor::Problem {
        sure_core::doctor::Problem {
            what: "SURE has recorded history on this machine and cannot read it.",
            detail: "the file is not a database".to_owned(),
        }
    }

    /// Every report this build can produce.
    fn every_report() -> Vec<Report> {
        vec![
            Report::Version,
            Report::Protocol,
            a_doctor_report(Vec::new()),
            a_doctor_report(vec![a_problem()]),
            Report::Unavailable(a_refusal()),
        ]
    }

    #[test]
    fn the_machine_form_is_one_line() {
        // A pipeline reading a stream of responses splits on newlines, so a
        // pretty-printed frame would be a frame and a half of somebody's output.
        for report in every_report() {
            let written = text(&report, true);
            assert!(
                !written.contains('\n'),
                "the machine form of {report:?} is not one line: {written}"
            );
        }
    }

    #[test]
    fn the_machine_form_names_the_build_and_the_protocol() {
        // The two facts that decide whether a captured response can still be
        // read. A frame without them is one a reader has to guess about.
        for report in every_report() {
            let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
            assert_eq!(
                frame["protocol_version"],
                json!(sure_core::PROTOCOL_VERSION)
            );
            assert_eq!(
                frame["sure_version"],
                json!(env!("CARGO_PKG_VERSION")),
                "the frame's version is not this build's"
            );
            assert_eq!(frame["command"], json!(report.command()));
            assert_eq!(frame["outcome"], json!(report.outcome()));
            assert_eq!(frame["exit_code"], json!(report.exit_code()));
        }
    }

    #[test]
    fn every_frame_carries_the_status_the_process_returns() {
        // Including the ones that are fine. A field that is present only when
        // something went wrong is a field every reader has to guard, and the
        // status is a fact about every run.
        for report in every_report() {
            let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
            assert!(
                frame["exit_code"].is_u64(),
                "{} produced a frame with no exit_code: {frame}",
                report.command()
            );
        }
    }

    #[test]
    fn a_doctor_report_that_found_a_problem_is_still_an_answer() {
        // The stream depends on whether SURE could do the thing, not on whether
        // the news is good. `sure doctor > report.txt` has to put the report in
        // the file; the status is what carries the bad news.
        let unhappy = a_doctor_report(vec![a_problem()]);
        assert!(unhappy.is_an_answer());
        assert_eq!(unhappy.outcome(), "not_green");
        assert_eq!(unhappy.exit_code(), exit::NOT_GREEN);
        assert_ne!(
            unhappy.exit_code(),
            exit::UNAVAILABLE,
            "a doctor that found a problem did not fail to run"
        );

        let happy = a_doctor_report(Vec::new());
        assert!(happy.is_an_answer());
        assert_eq!(happy.outcome(), "ok");
        assert_eq!(happy.exit_code(), exit::OK);
    }

    #[test]
    fn a_doctor_report_carries_what_it_found_in_one_place() {
        // The frame's fixed fields keep meaning what the module says they mean,
        // and everything a command found goes under `details`.
        let frame: serde_json::Value =
            serde_json::from_str(&text(&a_doctor_report(Vec::new()), true)).unwrap();
        let details = &frame["details"];
        assert!(details.is_object(), "doctor reported nothing: {frame}");
        for key in [
            "build",
            "places",
            "store",
            "tools",
            "problems",
            "not_checked",
        ] {
            assert!(
                details.get(key).is_some(),
                "a doctor report with no {key:?} is one a reader cannot use: {details}"
            );
        }
    }

    #[test]
    fn the_version_in_the_frame_is_the_one_the_version_command_prints() {
        // Two spellings of one fact. The prose says "SURE 0.0.0-bootstrap" and
        // the frame says "0.0.0-bootstrap", and if they were allowed to drift
        // the machine form would be the one that is wrong without anyone
        // noticing, because it is the one nobody reads.
        let printed = text(&Report::Version, false);
        let printed = printed.trim();
        assert!(
            printed.ends_with(env!("CARGO_PKG_VERSION")),
            "\"{printed}\" does not name this build"
        );
    }

    #[test]
    fn every_refusal_says_what_sure_did_instead_and_what_it_costs() {
        let written = text(&Report::Unavailable(a_refusal()), false);
        for needed in [
            "Nothing was checked.",
            "status 3",
            "rather than 0",
            "not implemented in this build",
        ] {
            assert!(
                written.contains(needed),
                "the refusal does not say {needed:?}:\n{written}"
            );
        }
    }

    #[test]
    fn an_answer_and_a_complaint_are_not_the_same_thing() {
        // The stream the human form takes. `sure check > report.txt` must leave
        // the complaint on the terminal.
        assert!(Report::Version.is_an_answer());
        assert!(Report::Protocol.is_an_answer());
        assert!(a_doctor_report(Vec::new()).is_an_answer());
        assert!(!Report::Unavailable(a_refusal()).is_an_answer());
    }

    #[test]
    fn nothing_that_is_unavailable_exits_zero() {
        // The whole point of the exit status. A command SURE cannot carry out
        // that exited 0 would be SURE's own false green, produced by the
        // program that exists to prevent them.
        let report = Report::Unavailable(a_refusal());
        assert_eq!(report.exit_code(), exit::UNAVAILABLE);
        assert_ne!(report.exit_code(), exit::OK);
        assert_eq!(report.outcome(), "unavailable");
    }

    #[test]
    fn every_outcome_name_is_one_of_the_documented_ones() {
        const DOCUMENTED: &[&str] = &["ok", "not_green", "unavailable"];
        for report in every_report() {
            assert!(
                DOCUMENTED.contains(&report.outcome()),
                "{} is not an outcome docs/architecture/CLI.md lists",
                report.outcome()
            );
        }
    }

    #[test]
    fn the_outcome_and_the_status_tell_the_same_story() {
        // Two spellings of one decision, and they must not drift. `ok` is the
        // only outcome that may exit 0; everything else stops a script.
        for report in every_report() {
            let clean = report.outcome() == "ok";
            assert_eq!(
                clean,
                report.exit_code() == exit::OK,
                "{} says {:?} and exits {}",
                report.command(),
                report.outcome(),
                report.exit_code()
            );
        }
    }
}

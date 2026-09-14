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
//! `command` and `outcome`. The first two are there so that a bug report
//! containing a captured response says which build produced it — the two things
//! that make a stored answer unreadable are a newer SURE and a newer protocol,
//! and a response that does not name either invites the reader to assume the
//! current ones.
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
    /// The command ran, and the answer is not clear.
    ///
    /// Reserved. `sure check` returns this once a project can be checked and
    /// findings are found, so that a pipeline can tell "the project has
    /// problems" from "SURE broke" — the two need opposite responses, and one
    /// status for both is how a broken tool gets read as a clean project.
    #[allow(
        dead_code,
        reason = "reserved by docs/architecture/CLI.md; no command can produce a verdict yet"
    )]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// `sure version`.
    Version,
    /// `sure protocol`.
    Protocol,
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
            Self::Unavailable(_) => "unavailable",
        }
    }

    /// The status this program exits with.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Version | Self::Protocol => exit::OK,
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
    /// `outcome()` is not the same question: once `sure check` can return a
    /// not-green verdict, that result is an *answer* — the verdict is the
    /// product, and the exit status is what carries the bad news.
    #[must_use]
    pub const fn is_an_answer(&self) -> bool {
        match self {
            Self::Version | Self::Protocol => true,
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
    fn frame(&self) -> serde_json::Value {
        let mut frame = json!({
            "sure_version": env!("CARGO_PKG_VERSION"),
            "protocol_version": sure_core::PROTOCOL_VERSION,
            "command": self.command(),
            "outcome": self.outcome(),
        });
        if let Self::Unavailable(not_yet) = self {
            frame["does"] = json!(not_yet.does);
            frame["instead"] = json!(not_yet.instead);
            frame["exit_code"] = json!(self.exit_code());
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

    #[test]
    fn the_machine_form_is_one_line() {
        // A pipeline reading a stream of responses splits on newlines, so a
        // pretty-printed frame would be a frame and a half of somebody's output.
        for report in [
            Report::Version,
            Report::Protocol,
            Report::Unavailable(a_refusal()),
        ] {
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
        for report in [
            Report::Version,
            Report::Protocol,
            Report::Unavailable(a_refusal()),
        ] {
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
    fn the_frame_carries_the_exit_status_the_process_returns() {
        // So that a script reading only the body, and a script reading only the
        // status, are told the same thing.
        let report = Report::Unavailable(a_refusal());
        let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
        assert_eq!(frame["exit_code"], json!(report.exit_code()));
    }

    #[test]
    fn every_outcome_name_is_one_of_the_documented_ones() {
        const DOCUMENTED: &[&str] = &["ok", "unavailable"];
        for report in [
            Report::Version,
            Report::Protocol,
            Report::Unavailable(a_refusal()),
        ] {
            assert!(
                DOCUMENTED.contains(&report.outcome()),
                "{} is not an outcome docs/architecture/CLI.md lists",
                report.outcome()
            );
        }
    }
}

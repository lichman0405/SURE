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

/// A goal SURE recorded, in a run that did not get as far as checking anything.
///
/// # Why this is a result of its own rather than a [`NotYet`]
///
/// Because something happened. `sure check --goal "…"` writes the goal down, and
/// a refusal that said only "not implemented in this build" would be true about
/// the check and false about the run: the user's history changed. A command
/// whose side effect is invisible is the shape of failure this program exists to
/// find, so the write is the first thing the report says.
///
/// # Why it still exits 3
///
/// The command the user asked for is `sure check`, and no check ran. Status 3 is
/// what stops a script that invoked it, and the frame carries the record so a
/// script can still see what was stored. Reporting success here would make
/// `sure check` exit 0 in CI while checking nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalRecorded {
    /// The goal, as the user gave it. Not a summary; see `cli.rs`.
    pub goal: String,
    /// The requirement's identifier, so that a later report can say whether the
    /// goal it compared against is the one this run recorded.
    pub requirement_id: String,
    /// Where the words came from.
    ///
    /// The domain's own enum rather than its wire name, so that the sentence a
    /// person reads and the field a script reads are two renderings of one
    /// value rather than two strings that have to be kept in step. The user
    /// asked for the goal; where it came from is what makes it usable as one,
    /// so it is a field rather than something the prose asserts on its own.
    pub source: sure_core::intent::IntentSource,
    /// The project the goal is about.
    pub project_root: String,
    /// The project state it was recorded against.
    ///
    /// The whole fingerprint rather than the identifier the store was handed,
    /// because the identifier is minted per run and means nothing on its own —
    /// what says *which* state this was is the kind and the digest. Both are
    /// printed, and the identifier goes in the frame for a script that wants to
    /// line this up with anything else the same run produced.
    pub project_state: sure_core::vocabulary::ProjectFingerprint,
    /// The store row it was written as.
    pub record: i64,
}

/// A command that tried and did not finish.
///
/// The counterpart to [`NotYet`]: that one is a command this build cannot carry
/// out at all, and this one is a command that ran, met something it could not
/// get past, and stopped. The two are different answers — the first is a newer
/// build, the second is a bug report or a broken installation — which is why
/// they are different variants and different exit statuses.
///
/// `what` is a `&'static str` for [`NotYet`]'s reason: a sentence SURE says about
/// itself is fixed, so no call site can paraphrase it into something weaker.
/// `detail` is the underlying error's own message, which is where project- and
/// machine-controlled text belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    /// The command, from the grammar rather than retyped — see `Command::name`.
    pub command: &'static str,
    /// What did not happen, in a fixed sentence that says nothing was written
    /// when that is so.
    pub what: &'static str,
    /// What went wrong, in the words of the thing that went wrong.
    pub detail: String,
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
    /// `sure protocol --speaks N`, carrying what SURE can say to that caller.
    Handshake(sure_core::Handshake),
    /// `sure doctor`, carrying what it found.
    Doctor(Box<sure_core::doctor::DoctorReport>),
    /// `sure check --goal`, which recorded the goal and stopped before it could
    /// check anything.
    GoalRecorded(Box<GoalRecorded>),
    /// A command whose work lands in a later phase.
    Unavailable(NotYet),
    /// A command that tried and did not finish.
    Failed(Box<Failed>),
    /// `sure hook ingest`, carrying a protection decision for a pre-action hook.
    ///
    /// The machine form is the decision JSON that the harness reads from stdout.
    HookDecision(sure_core::hook_protection::ProtectionDecision),
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
            // A refusal to speak a caller's protocol is not this build failing;
            // it is this build saying it cannot carry that out, which is what
            // `unavailable` means everywhere else on this surface. The message
            // says which side has to change.
            Self::Handshake(handshake) => {
                if handshake.is_agreed() {
                    "ok"
                } else {
                    "unavailable"
                }
            }
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
            // The command the user asked for did not happen, which is what
            // `unavailable` means on this surface. Not `ok`: nothing was checked.
            // Not `failed`: nothing went wrong, and a record was written that
            // this outcome tells a script to go looking for under `details`.
            Self::GoalRecorded(_) => "unavailable",
            Self::Unavailable(_) => "unavailable",
            // A command that did not finish has not answered, and saying
            // `not_green` would read as "the project has problems" — which is a
            // statement about a project SURE never got to look at.
            Self::Failed(_) => "failed",
            // A hook decision is an answer: the command ran and produced a result.
            // `not_green` for block because the answer is "do not proceed".
            Self::HookDecision(decision) => match decision.decision {
                sure_core::hook_protection::ProtectionDecisionKind::Allow
                | sure_core::hook_protection::ProtectionDecisionKind::Warn => "ok",
                sure_core::hook_protection::ProtectionDecisionKind::Block => "not_green",
            },
        }
    }

    /// The status this program exits with.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Version | Self::Protocol => exit::OK,
            // 3, not 4. `docs/architecture/CLI.md` reserves 4 for a refusal that
            // is a *decision about authority* — a project asking for privileges
            // it cannot be granted — where the user is the one who changes the
            // answer. This is not a decision about anything the user configured:
            // it is this build saying it cannot speak that protocol, and for one
            // of the two directions the remedy is literally a newer build, which
            // is what 3 is documented to mean.
            Self::Handshake(handshake) => {
                if handshake.is_agreed() {
                    exit::OK
                } else {
                    exit::UNAVAILABLE
                }
            }
            Self::Doctor(report) => {
                if report.is_well() {
                    exit::OK
                } else {
                    exit::NOT_GREEN
                }
            }
            // 3, not 0 and not 5. The check is the thing the user asked for and
            // it did not run — which is what 3 means — and nothing went wrong.
            // The record the goal was written as is in the report, so a script
            // is not left thinking the run had no effect.
            Self::GoalRecorded(_) => exit::UNAVAILABLE,
            Self::Unavailable(_) => exit::UNAVAILABLE,
            Self::Failed(_) => exit::FAILED,
            // 0 for allow/warn so the launcher does not block the operation.
            // 1 for block so the launcher can relay the refusal.
            Self::HookDecision(decision) => match decision.decision {
                sure_core::hook_protection::ProtectionDecisionKind::Allow
                | sure_core::hook_protection::ProtectionDecisionKind::Warn => exit::OK,
                sure_core::hook_protection::ProtectionDecisionKind::Block => exit::NOT_GREEN,
            },
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
    ///
    /// A handshake that did not agree is a complaint by that rule: there is no
    /// report to put in a file, and a caller that piped the output somewhere
    /// meant to read whether the two can talk. The *machine* form still goes to
    /// stdout, because a script asked for it in so many words.
    #[must_use]
    pub const fn is_an_answer(&self) -> bool {
        match self {
            Self::Version | Self::Protocol | Self::Doctor(_) => true,
            Self::Handshake(handshake) => handshake.is_agreed(),
            // `sure check --goal > report.txt` has to leave the complaint on the
            // terminal. The run recorded a goal and checked nothing, which is not
            // a report about the project, and a file holding only the recording
            // would read as one.
            Self::GoalRecorded(_) | Self::Unavailable(_) | Self::Failed(_) => false,
            // A hook decision is an answer: the command ran and produced a result.
            Self::HookDecision(_) => true,
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
            // this build, and nothing else.
            Self::Protocol => writeln!(
                out,
                "Harness protocol version {}.",
                sure_core::PROTOCOL_VERSION
            ),
            // One sentence, whichever way it went, and it is the handshake's own
            // — the same sentence the event reader's refusal builds on. A
            // renderer that wrote its own could word the same answer another
            // way, and the two would then be two explanations of one rule.
            Self::Handshake(handshake) => writeln!(out, "{handshake}"),
            Self::Doctor(report) => crate::doctor::human(report, out),
            // What happened first, then what did not. A person who ran this has
            // a record in their history now, and the report opens by saying so
            // rather than burying it under the refusal.
            Self::GoalRecorded(recorded) => {
                writeln!(out, "Your goal was recorded, and nothing was checked.")?;
                writeln!(out)?;
                writeln!(out, "  what you asked for  {}", recorded.goal)?;
                // The domain's own sentence about what this label is worth,
                // rather than a second wording written here. A renderer that
                // paraphrased the trust label could weaken it without anything
                // failing.
                writeln!(
                    out,
                    "  where it came from  {}",
                    recorded.source.plain_description()
                )?;
                writeln!(out, "  project             {}", recorded.project_root)?;
                writeln!(
                    out,
                    "  project state       {} ({} fingerprint)",
                    recorded.project_state.digest,
                    recorded.project_state.kind.as_str()
                )?;
                writeln!(out, "  record              #{}", recorded.record)?;
                writeln!(out)?;
                writeln!(
                    out,
                    "SURE read the project only far enough to say which state of it the goal \
                     was recorded against, because this build cannot check a project yet. Your \
                     history has the goal in it; the project itself is unchanged."
                )?;
                writeln!(out)?;
                writeln!(
                    out,
                    "SURE exited with status {} rather than {}, because work that was not done \
                     and work that succeeded must never look alike to a script.",
                    exit::UNAVAILABLE,
                    exit::OK
                )
            }
            Self::Failed(failure) => {
                writeln!(
                    out,
                    "sure {command} could not finish.",
                    command = failure.command
                )?;
                writeln!(out)?;
                writeln!(out, "{}", failure.what)?;
                writeln!(out)?;
                writeln!(out, "{}", failure.detail)?;
                writeln!(out)?;
                writeln!(
                    out,
                    "SURE exited with status {}, which is what it returns when it tried and did \
                     not finish. That is not the same as a command this build cannot carry out, \
                     and not the same as a wrong command line.",
                    exit::FAILED
                )
            }
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
            Self::HookDecision(decision) => match decision.decision {
                sure_core::hook_protection::ProtectionDecisionKind::Allow => {
                    writeln!(out, "SURE allows this tool request.")
                }
                sure_core::hook_protection::ProtectionDecisionKind::Warn => {
                    writeln!(
                        out,
                        "SURE advises caution: {}",
                        decision
                            .reason
                            .as_deref()
                            .unwrap_or("This action needs approval.")
                    )
                }
                sure_core::hook_protection::ProtectionDecisionKind::Block => {
                    writeln!(
                        out,
                        "SURE blocks this tool request: {}",
                        decision
                            .reason
                            .as_deref()
                            .unwrap_or("The current execution mode does not permit this action.")
                    )
                }
            },
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
            // A caller that asked whether it can talk needs the two numbers side
            // by side and which side has to move. Deliberately no sentence here:
            // the frame is what a script reads, and a script that acts on prose
            // is a script that breaks when the prose is improved.
            Self::Handshake(handshake) => {
                frame["details"] = json!({
                    "sure_speaks": handshake.sure_speaks(),
                    "caller_speaks": handshake.caller_speaks(),
                    "agreed": handshake.is_agreed(),
                    "update": match handshake {
                        sure_core::Handshake::Agreed { .. } => None,
                        sure_core::Handshake::CallerIsOlder { .. } => Some("caller"),
                        sure_core::Handshake::CallerIsNewer { .. } => Some("sure"),
                    },
                });
            }
            // What was stored, for a script that wants to act on it rather than
            // only notice it. `source` is the wire name here, not the sentence:
            // the frame is what a reader switches on, and a switch on prose
            // breaks when the prose is improved.
            Self::GoalRecorded(recorded) => {
                let state = &recorded.project_state;
                frame["details"] = json!({
                    "goal": recorded.goal,
                    "requirement_id": recorded.requirement_id,
                    "source": recorded.source.as_str(),
                    "project_root": recorded.project_root,
                    // Written out field by field rather than by serializing the
                    // domain type, so that the shape a script reads is decided
                    // here. A field added to that type must not appear in this
                    // frame without somebody choosing to put it there.
                    "project_state": {
                        "id": state.id.as_str(),
                        "kind": state.kind.as_str(),
                        "digest": state.digest,
                    },
                    "record": recorded.record,
                });
            }
            // The failure's own words. `what` is a sentence SURE wrote about
            // itself and `detail` is whatever went wrong, kept apart here for
            // the same reason they are apart in the struct: a reader deciding
            // whether to file a bug should not have to parse prose to find out.
            Self::Failed(failure) => {
                frame["details"] = json!({
                    "what": failure.what,
                    "detail": failure.detail,
                });
            }
            Self::Version | Self::Protocol => {}
            Self::HookDecision(decision) => {
                frame["decision"] = json!(decision.decision.as_str());
                if let Some(ref reason) = decision.reason {
                    frame["reason"] = json!(reason);
                }
            }
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
            Self::Protocol | Self::Handshake(_) => "protocol",
            Self::Doctor(_) => "doctor",
            // The command the user typed. `sure check --goal "…"` is `check`,
            // and a frame saying otherwise would name a command that does not
            // exist on this surface.
            Self::GoalRecorded(_) => "check",
            Self::Unavailable(not_yet) => not_yet.command,
            Self::Failed(failure) => failure.command,
            Self::HookDecision(_) => "hook",
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

    /// A goal that was recorded in a run that checked nothing.
    fn a_recorded_goal() -> Report {
        Report::GoalRecorded(Box::new(GoalRecorded {
            goal: "make the upload reject a file over 10 MB".to_owned(),
            requirement_id: "goal".to_owned(),
            source: sure_core::intent::IntentSource::ExplicitUserGoal,
            project_root: "C:\\work\\thing".to_owned(),
            project_state: sure_core::vocabulary::ProjectFingerprint::content("2f9c1a04"),
            record: 7,
        }))
    }

    /// A command that tried and did not finish.
    fn a_failure() -> Report {
        Report::Failed(Box::new(Failed {
            command: "check",
            what: "Nothing was recorded and nothing was checked.",
            detail: "the store is locked by another process".to_owned(),
        }))
    }

    /// Every report this build can produce.
    ///
    /// Every *shape* of report, which is what the frame, the outcome and the
    /// stream are decided by — not every value any of them can carry. The one
    /// shape with a value worth varying is the handshake, where both directions
    /// are here because both have to be renderable.
    fn every_report() -> Vec<Report> {
        vec![
            Report::Version,
            Report::Protocol,
            Report::Handshake(sure_core::negotiate(sure_core::PROTOCOL_VERSION)),
            Report::Handshake(sure_core::negotiate(sure_core::PROTOCOL_VERSION + 1)),
            Report::Handshake(sure_core::negotiate(0)),
            a_doctor_report(Vec::new()),
            a_doctor_report(vec![a_problem()]),
            a_recorded_goal(),
            a_failure(),
            Report::Unavailable(a_refusal()),
            Report::HookDecision(sure_core::hook_protection::ProtectionDecision::allow()),
            Report::HookDecision(sure_core::hook_protection::ProtectionDecision::warn(
                "This action needs explicit approval before it can run.",
            )),
            Report::HookDecision(sure_core::hook_protection::ProtectionDecision::block(
                "The current execution mode does not permit this action.",
            )),
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
        assert!(
            Report::Handshake(sure_core::negotiate(sure_core::PROTOCOL_VERSION)).is_an_answer()
        );
        assert!(
            !Report::Handshake(sure_core::negotiate(sure_core::PROTOCOL_VERSION + 1))
                .is_an_answer()
        );
        assert!(!Report::Unavailable(a_refusal()).is_an_answer());
        // A run that recorded a goal and checked nothing has no report about
        // the project to put in a file.
        assert!(!a_recorded_goal().is_an_answer());
        assert!(!a_failure().is_an_answer());
    }

    #[test]
    fn a_recorded_goal_is_a_result_of_its_own_and_not_a_refusal() {
        // The reason this variant exists. The command did something: the user's
        // history changed. A report that only said "not implemented in this
        // build" would be true about the check and false about the run, and the
        // shape of failure this program exists to find is a side effect nobody
        // was told about.
        let report = a_recorded_goal();

        // Still not a success: `sure check` in CI must not exit 0 while
        // checking nothing.
        assert_eq!(report.exit_code(), exit::UNAVAILABLE);
        assert_ne!(report.exit_code(), exit::OK);
        assert_ne!(
            report.exit_code(),
            exit::FAILED,
            "nothing went wrong, so this is not the status for a failed run"
        );
        assert_eq!(report.outcome(), "unavailable");
        assert_eq!(report.command(), "check");

        let written = text(&report, false);
        assert!(
            written.contains("make the upload reject a file over 10 MB"),
            "the report does not say what was recorded:\n{written}"
        );
        assert!(
            written.contains("recorded") && written.contains("nothing was checked"),
            "the report buries what happened:\n{written}"
        );
        // The domain's own sentence about the label, not a second wording.
        assert!(
            written.contains(sure_core::intent::IntentSource::ExplicitUserGoal.plain_description()),
            "the report does not say what the source is worth:\n{written}"
        );

        let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
        let details = &frame["details"];
        assert_eq!(details["record"], json!(7));
        assert_eq!(details["requirement_id"], json!("goal"));
        // The wire name in the frame, the sentence in the prose: one value, two
        // renderings, and the reader switches on the machine one.
        assert_eq!(details["source"], json!("explicit_user_goal"));
        // Which state, in the two fields that answer it: a digest two runs can
        // compare, and the kind that says how it was computed.
        assert_eq!(details["project_state"]["digest"], json!("2f9c1a04"));
        assert_eq!(details["project_state"]["kind"], json!("content"));
        assert!(
            details["project_state"]["id"]
                .as_str()
                .is_some_and(|id| !id.is_empty()),
            "the run's own fingerprint identity is missing from the frame: {details}"
        );
    }

    #[test]
    fn a_command_that_did_not_finish_is_not_a_command_that_cannot_be_carried_out() {
        // Two answers that look alike from a distance and need opposite
        // responses: "wait for a newer build" and "something is broken, here is
        // what". One status for both is how a broken tool gets read as a tool
        // with nothing to do.
        let report = a_failure();

        assert_eq!(report.exit_code(), exit::FAILED);
        assert_ne!(report.exit_code(), exit::UNAVAILABLE);
        assert_ne!(report.exit_code(), exit::OK);
        assert_eq!(report.outcome(), "failed");
        assert_eq!(report.command(), "check");

        let written = text(&report, false);
        assert!(
            written.contains("the store is locked by another process"),
            "the failure does not carry what went wrong:\n{written}"
        );
        assert!(
            written.contains("Nothing was recorded and nothing was checked."),
            "the failure does not say what did not happen:\n{written}"
        );
        assert!(
            written.contains(&exit::FAILED.to_string()),
            "the failure does not say what it exits with:\n{written}"
        );
        assert!(
            !written.contains("not implemented in this build"),
            "a run that tried must not be worded as a command this build lacks:\n{written}"
        );

        let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
        assert_eq!(
            frame["details"]["detail"],
            json!("the store is locked by another process")
        );
        assert_eq!(
            frame["details"]["what"],
            json!("Nothing was recorded and nothing was checked.")
        );
    }

    #[test]
    fn a_handshake_that_did_not_agree_stops_a_caller_and_says_which_side_moves() {
        // The reason the handshake exists: an adapter that is told "no" has to
        // be able to tell whether to update itself or SURE, and it has to be
        // stopped from sending anything in the meantime. Status 3 is what stops
        // it, and it is not 0.
        for caller in [0, sure_core::PROTOCOL_VERSION + 1] {
            let report = Report::Handshake(sure_core::negotiate(caller));
            assert_eq!(report.exit_code(), exit::UNAVAILABLE);
            assert_ne!(report.exit_code(), exit::OK);
            assert_eq!(report.outcome(), "unavailable");
            assert!(!report.is_an_answer());
            assert_eq!(report.command(), "protocol");

            let sentence = text(&report, false);
            assert!(
                sentence.contains(&caller.to_string()),
                "the caller is not told what it spoke: {sentence}"
            );
            assert!(
                sentence.contains(&format!("speaks {}", sure_core::PROTOCOL_VERSION)),
                "the caller is not told what SURE speaks: {sentence}"
            );

            let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
            let expected = if caller < sure_core::PROTOCOL_VERSION {
                "caller"
            } else {
                "sure"
            };
            assert_eq!(
                frame["details"]["update"], expected,
                "the frame does not say which side has to move: {frame}"
            );
            assert_eq!(frame["details"]["caller_speaks"], json!(caller));
            assert_eq!(frame["details"]["agreed"], json!(false));
        }
    }

    #[test]
    fn a_handshake_that_agreed_is_an_answer_a_caller_can_act_on() {
        // The other half, and the one a working adapter sees on every run: exit
        // 0, on stdout, with both numbers present so that a caller can check
        // what it was told rather than take it on faith.
        let report = Report::Handshake(sure_core::negotiate(sure_core::PROTOCOL_VERSION));

        assert_eq!(report.exit_code(), exit::OK);
        assert_eq!(report.outcome(), "ok");
        assert!(report.is_an_answer());

        let frame: serde_json::Value = serde_json::from_str(&text(&report, true)).unwrap();
        assert_eq!(frame["details"]["agreed"], json!(true));
        assert_eq!(
            frame["details"]["sure_speaks"],
            json!(sure_core::PROTOCOL_VERSION)
        );
        assert!(
            frame["details"]["update"].is_null(),
            "an agreement says nothing has to be updated: {frame}"
        );
        assert!(
            frame["details"]["caller_speaks"].is_null(),
            "the agreed version is SURE's own and is already in the frame: {frame}"
        );
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
        const DOCUMENTED: &[&str] = &["ok", "not_green", "unavailable", "failed"];
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

//! What each command does in this build.
//!
//! # One match, and no default arm
//!
//! [`Command::report`] is the only dispatch in the program, and it matches every
//! variant of `Command` by name. There is no `_ =>` and no `unreachable!()`.
//! That is the whole design: a command added to the grammar stops this file
//! compiling until somebody decides whether this build implements it, and the
//! answer is either a real result or a [`NotYet`].
//!
//! The alternative — a default arm returning "not implemented" — would let a
//! command ship as a stub that nobody chose to make a stub, and `sure check`
//! answering "not implemented" forever would look exactly like `sure check`
//! answering "not implemented until P2".
//!
//! # Where the command names come from
//!
//! [`not_yet`] takes the `&Command` rather than a name, and asks it. A refusal
//! that retyped "check" could name a command other than the one the user typed,
//! and nothing would catch it. The one command whose name is not its own is
//! `history`, which can be asked to do things with different consequences; see
//! [`history_name`].
//!
//! # Why so few commands work
//!
//! Three do, and all three answer questions about SURE or about this machine
//! rather than about a project: `version`, `protocol` and `doctor`. Every other
//! command needs the engine, and the engine is what the phases after this one
//! build. Saying so is not a gap in this task; it is the task.
//! `docs/architecture/CLI.md` records what each command is for, and this file is
//! where the ones that cannot run say so to the user rather than to a reader of
//! the docs.

use crate::cli::{Command, HistoryAction};
use crate::report::{NotYet, Report};

impl Command {
    /// What SURE does about this command, in this build.
    #[must_use]
    pub fn report(&self) -> Report {
        match self {
            // The three this build can answer. All three are questions about
            // SURE or about this machine rather than about a project, which is
            // why none of them needs a check engine.
            Self::Version => Report::Version,
            Self::Protocol => Report::Protocol,
            // The examination happens here rather than in `Report`, so that the
            // report stays a value — something a test can build and a renderer
            // can read — instead of a thing that goes and looks.
            Self::Doctor => Report::Doctor(Box::new(sure_core::doctor::examine_this_machine())),

            // Everything below is a real command with a real job and no
            // implementation yet.
            Self::Check { .. } => not_yet(
                self,
                "check a project and report what it found",
                "Nothing was checked.",
            ),
            Self::Recheck { .. } => not_yet(
                self,
                "check a project again and compare what it finds with what it found last time",
                "Nothing was checked, and nothing was compared.",
            ),
            Self::Repair { .. } => not_yet(
                self,
                "turn what was found into instructions a coding agent can act on",
                "No repair instructions were produced.",
            ),
            Self::History { action } => Report::Unavailable(NotYet {
                command: history_name(action),
                does: "show what SURE has recorded, on this machine and for this project",
                instead: "Nothing was read from the history, and nothing was deleted.",
            }),
            Self::Config { .. } => not_yet(
                self,
                "show the settings in effect and which layer each one came from",
                "No configuration was read.",
            ),
            Self::Hook { .. } => not_yet(
                self,
                "record one event from a coding harness",
                "The event was not read from standard input and was not recorded.",
            ),
            Self::Explain { .. } => not_yet(
                self,
                "explain one recorded result in plain language",
                "Nothing was explained.",
            ),
        }
    }
}

/// Build the refusal for one command.
fn not_yet(command: &Command, does: &'static str, instead: &'static str) -> Report {
    Report::Unavailable(NotYet {
        command: command.name(),
        does,
        instead,
    })
}

/// The name a user sees for a `sure history` invocation.
///
/// The subcommand is part of that name, and it has to be: a user who typed the
/// destructive one and is answered about the harmless one has been told the
/// wrong thing. Every arm is a literal, so nothing a project controls can reach
/// this string.
fn history_name(action: &Option<HistoryAction>) -> &'static str {
    match action {
        None | Some(HistoryAction::List) => "history",
        Some(HistoryAction::Show { .. }) => "history show",
        Some(HistoryAction::Delete) => "history delete",
        Some(HistoryAction::Export) => "history export",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cli::{ConfigAction, HookAction};

    /// Every invocation the grammar accepts, built by hand.
    ///
    /// By hand rather than by walking clap's `CommandFactory`, because this is
    /// the list of *commands* and a list derived from the parser would share a
    /// source of truth with the thing it checks.
    fn every_command() -> Vec<Command> {
        vec![
            Command::Check { path: None },
            Command::Recheck { path: None },
            Command::Repair { path: None },
            Command::History { action: None },
            Command::History {
                action: Some(HistoryAction::List),
            },
            Command::History {
                action: Some(HistoryAction::Show { id: None }),
            },
            Command::History {
                action: Some(HistoryAction::Delete),
            },
            Command::History {
                action: Some(HistoryAction::Export),
            },
            Command::Doctor,
            Command::Config { action: None },
            Command::Config {
                action: Some(ConfigAction::Paths),
            },
            Command::Config {
                action: Some(ConfigAction::Show),
            },
            Command::Config {
                action: Some(ConfigAction::Validate),
            },
            Command::Hook {
                action: HookAction::Ingest,
            },
            Command::Explain { id: None },
            Command::Protocol,
            Command::Version,
        ]
    }

    #[test]
    fn every_command_this_build_cannot_carry_out_says_so_and_exits_non_zero() {
        // The load-bearing test of this task. A command that reached a user as
        // a success without doing anything is SURE's own false green, and SURE
        // is the program that exists to find those.
        for command in every_command() {
            let report = command.report();
            match &report {
                Report::Unavailable(not_yet) => {
                    assert_eq!(
                        not_yet.command.split(' ').next(),
                        Some(command.name()),
                        "{command:?} is refused under a different name than it was asked by"
                    );
                    assert_eq!(report.exit_code(), crate::report::exit::UNAVAILABLE);
                    assert_ne!(report.exit_code(), crate::report::exit::OK);
                    assert!(
                        !not_yet.does.is_empty(),
                        "{command:?} says nothing about itself"
                    );
                    assert!(
                        !not_yet.instead.is_empty(),
                        "{command:?} does not say what SURE did instead"
                    );
                    assert!(
                        not_yet.instead.ends_with('.'),
                        "{command:?} does not finish its sentence: {:?}",
                        not_yet.instead
                    );
                }
                Report::Version | Report::Protocol | Report::Doctor(_) => {}
            }
        }
    }

    #[test]
    fn the_commands_this_build_implements_are_exactly_these_three() {
        // Stated as a list, so that making a fourth command work is a change to
        // this test rather than a side effect somebody notices later. If a
        // command starts working without this list moving, something returned a
        // success it had not earned.
        let implemented: Vec<&str> = every_command()
            .iter()
            .filter(|command| !matches!(command.report(), Report::Unavailable(_)))
            .map(Command::name)
            .collect();
        assert_eq!(implemented, ["doctor", "protocol", "version"]);
    }

    #[test]
    fn a_command_that_runs_never_answers_a_question_it_was_not_asked() {
        // The refusals are held to the command they were asked by; the commands
        // that work have to be held to the same rule, and `Report::command` is
        // where that name lives. A `sure doctor` frame that said `version` would
        // be a script reading the wrong answer.
        //
        // The first word has to match, not the whole name: `sure history delete`
        // answers as "history delete", because the subcommand is part of what
        // the user asked for and a refusal that dropped it would be answering
        // about the wrong thing. See [`history_name`].
        for command in every_command() {
            assert_eq!(
                command.report().command().split(' ').next(),
                Some(command.name()),
                "{command:?} answers under a different name than it was asked by"
            );
        }
    }

    #[test]
    fn a_doctor_that_found_a_problem_does_not_exit_zero() {
        // The false-green rule applied to this machine's own state. The core
        // decides what a problem is; what this asserts is that the CLI cannot
        // turn one into a success on the way out.
        let report = Command::Doctor.report();
        let Report::Doctor(doctor) = &report else {
            panic!("sure doctor is implemented");
        };
        if doctor.is_well() {
            assert_eq!(report.exit_code(), crate::report::exit::OK);
        } else {
            assert_eq!(report.exit_code(), crate::report::exit::NOT_GREEN);
            assert_ne!(report.exit_code(), crate::report::exit::OK);
        }
    }

    #[test]
    fn a_history_subcommand_is_refused_under_its_own_name() {
        // A user who typed the destructive one must not be answered about the
        // harmless one.
        for (action, expected) in [
            (None, "history"),
            (Some(HistoryAction::List), "history"),
            (Some(HistoryAction::Show { id: None }), "history show"),
            (Some(HistoryAction::Delete), "history delete"),
            (Some(HistoryAction::Export), "history export"),
        ] {
            assert_eq!(history_name(&action), expected);
        }
    }

    #[test]
    fn the_refusal_for_a_destructive_subcommand_does_not_claim_a_deletion_happened() {
        // The user's next question after `sure history delete` fails is whether
        // it worked anyway, so the message has to answer it in those words.
        let deleting = Command::History {
            action: Some(HistoryAction::Delete),
        };
        let refusal = match deleting.report() {
            Report::Unavailable(not_yet) => not_yet,
            other => panic!("expected a refusal, got {other:?}"),
        };
        assert!(refusal.instead.contains("nothing was deleted"));
    }
}

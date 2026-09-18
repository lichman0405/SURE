//! The command surface, as a grammar.
//!
//! `docs/architecture/CLI.md` is the authority for what these commands mean and
//! which phase implements each one. This module is only the shape: what SURE
//! accepts, and what it refuses before a command body is ever reached.
//!
//! # Why the grammar is one type
//!
//! [`Command`] is the whole surface in one enum, and
//! [`Command::report`](crate::commands::Command::report) matches on it
//! exhaustively. Adding a command to the grammar therefore cannot leave the
//! build compiling with no answer about whether this build implements it: the
//! match stops compiling until the new arm says.
//!
//! # What is deliberately not here
//!
//! No command carries a flag that only a later phase can honour. A grammar that
//! accepts `--deep` and does nothing with it is the CLI teaching a user that
//! SURE understood them when it did not, and the user has no way to tell.
//! Arguments arrive with the phase that can act on them.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::output::Format;

/// `sure`.
#[derive(Debug, Parser)]
#[command(
    name = "sure",
    version,
    about = "SURE — Software Understanding & Reality Evaluation",
    long_about = "AI says it's done. Be SURE.\n\n\
                  SURE checks what was actually built, records what it found, and says what it \
                  could not confirm. Every result is a record with evidence behind it, or it is \
                  not reported.",
    // A bare `sure` did nothing, so it is not a success. clap writes the help
    // to stderr and exits 2, which is the same answer a wrong flag gets.
    arg_required_else_help = true
)]
pub struct Cli {
    /// How to write results. `human` is for reading, `json` is for scripts.
    ///
    /// Global, so it is accepted before or after the command name. It is the
    /// only place the choice is made: nothing else in this binary decides which
    /// of the two output paths runs, and
    /// `tests/cli_contract.rs` checks that no other module writes to stdout.
    #[arg(long, value_enum, default_value_t = Format::Human, global = true)]
    pub format: Format,

    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Every command SURE accepts.
///
/// `docs/architecture/CLI.md` lists the same set in the same order. The doc and
/// this enum are two views of one list, and `tests/cli_contract.rs` checks they
/// agree by asking this build what it parses.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check a project and report what was found.
    Check {
        /// The project to check. Default: the current directory.
        path: Option<PathBuf>,

        /// What you want the project to do, in your own words.
        ///
        /// This is the one channel the project cannot write, and the reason it
        /// exists: a goal found in a README or in `sure.yaml` is documentation,
        /// because the agent whose work is being checked can edit both. A goal
        /// typed here is a user requirement, and SURE may compare the project
        /// against it — see `docs/architecture/PROJECT_INTENT.md`.
        ///
        /// The wording is kept exactly as given. SURE does not summarize it,
        /// rewrap it or trim it, because a goal that lost a clause on the way in
        /// is a requirement the user never stated.
        #[arg(long, value_name = "TEXT")]
        goal: Option<String>,
    },

    /// Check a project again, after a repair, and compare with last time.
    Recheck {
        /// The project to re-check. Default: the current directory.
        path: Option<PathBuf>,
    },

    /// Turn what was found into instructions a coding agent can act on.
    Repair {
        /// The project to produce repair instructions for. Default: the
        /// current directory.
        path: Option<PathBuf>,
    },

    /// Look at what SURE has recorded.
    History {
        /// What to do with the history. Default: `list`.
        #[command(subcommand)]
        action: Option<HistoryAction>,
    },

    /// Report on this machine: where SURE keeps things, and what it found.
    Doctor,

    /// Look at the configuration SURE is using, and where it came from.
    Config {
        /// What to show. Default: a summary of every layer.
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// The entry points a coding harness calls.
    Hook {
        /// What the harness wants SURE to do.
        #[command(subcommand)]
        action: HookAction,
    },

    /// Explain one recorded result in plain language.
    Explain {
        /// The result to explain.
        id: Option<String>,
    },

    /// Print the version of the harness protocol this build speaks.
    Protocol {
        /// Check whether this build can talk to a caller that speaks this
        /// protocol version.
        ///
        /// A caller that already knows what it was built against asks with
        /// this, and SURE answers — so that the rule about which versions can
        /// talk lives in one place instead of in every adapter's language. See
        /// `sure_protocol::handshake`.
        #[arg(long, value_name = "VERSION")]
        speaks: Option<u32>,
    },

    /// Print the version of this build.
    Version,
}

/// What `sure history` can be asked to do.
#[derive(Debug, Subcommand)]
pub enum HistoryAction {
    /// List what is recorded.
    List,
    /// Show one record.
    Show {
        /// The record to show.
        id: Option<String>,
    },
    /// Delete records. Long-term history only; the store refuses to delete
    /// anything else.
    Delete,
    /// Write the history out as JSON.
    Export,
}

/// What `sure config` can be asked to do.
#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show where SURE keeps its files on this machine.
    Paths,
    /// Show the settings in effect and which layer each came from.
    Show,
    /// Report problems in the configuration without checking the project.
    Validate,
}

/// What a harness can ask `sure hook` to do.
#[derive(Debug, Subcommand)]
pub enum HookAction {
    /// Take one harness event and record it.
    ///
    /// The harness writes JSON to stdin. The event is normalised, validated,
    /// and persisted. For `pre-tool-use` events a protection decision is also
    /// evaluated and written to stdout as JSON.
    Ingest {
        /// Which harness sent the event.
        #[arg(long, value_name = "NAME")]
        source: Option<String>,

        /// The kind of event, as named by the harness.
        event_kind: Option<String>,
    },
}

impl Command {
    /// The command as a user would type it.
    ///
    /// Used for the `command` field of the machine-readable frame and for the
    /// first line of a refusal, so the two cannot name different things.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Check { .. } => "check",
            Self::Recheck { .. } => "recheck",
            Self::Repair { .. } => "repair",
            Self::History { .. } => "history",
            Self::Doctor => "doctor",
            Self::Config { .. } => "config",
            Self::Hook { .. } => "hook",
            Self::Explain { .. } => "explain",
            Self::Protocol { .. } => "protocol",
            Self::Version => "version",
        }
    }
}

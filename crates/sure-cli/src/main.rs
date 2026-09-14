#![forbid(unsafe_code)]
//! The `sure` command line entry point.
//!
//! The command framework lands in P1-T008. Until then this binary exposes only
//! what is already true, so that nothing here claims a capability the core does
//! not yet have.

use std::process::ExitCode;

const USAGE: &str = "\
SURE — Software Understanding & Reality Evaluation
AI says it's done. Be SURE.

Usage:
  sure version      Show the version
  sure help         Show this message

This build is an early development snapshot. `sure check` and the other
commands arrive in later phases of the task graph.";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("version" | "--version" | "-V") => {
            println!("{}", sure_core::version_string());
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        None => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("sure: unknown command '{other}'");
            eprintln!();
            eprintln!("{USAGE}");
            // An unrecognised command is a user error, not a successful run.
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_usage_text_leads_with_the_product_promise() {
        assert!(USAGE.contains(sure_core::PROMISE));
    }

    #[test]
    fn the_usage_text_does_not_list_commands_that_do_not_exist_yet() {
        // These are real product commands, intentionally absent from the usage
        // list until the command framework lands in P1-T008. Advertising one
        // before it exists would be the CLI telling a user something untrue.
        //
        // Only the indented usage lines count: the prose paragraph above them
        // names `sure check` on purpose, to say it is not here yet.
        let listed: Vec<&str> = USAGE
            .lines()
            .filter(|line| line.starts_with("  sure "))
            .collect();
        assert!(!listed.is_empty(), "usage must list at least one command");
        for line in listed {
            let command = line.split_whitespace().nth(1).unwrap_or_default();
            assert!(
                matches!(command, "version" | "help"),
                "'sure {command}' is advertised in usage but is not implemented yet"
            );
        }
    }
}

//! The two output paths, and the rule that keeps them apart.
//!
//! # The rule
//!
//! There are exactly two ways this program writes a result, and the format the
//! user asked for decides which one runs:
//!
//! | Format | Stream | Shape |
//! | --- | --- | --- |
//! | `human` | the stream that matches the outcome — see [`Format::emit`] | prose for a person |
//! | `json` | always stdout | one line, one object |
//!
//! The machine path always uses stdout, including when the command failed,
//! because the JSON *is* the result: a script that asked for the failure reason
//! has to be able to read it. The human path uses stdout for an answer and
//! stderr for a complaint, so that `sure check > report.txt` puts the report in
//! the file and leaves the complaint on the terminal.
//!
//! # Why it is one module
//!
//! "Human and machine-readable output are separated" is not satisfied by
//! having two renderers. It is satisfied by their being no third way out.
//! This module is the only place in the crate that names `io::stdout` or
//! `io::stderr`, so a stray `println!` added anywhere else is the one thing
//! that could break the separation — and
//! `tests/cli_contract.rs::only_the_output_module_writes_to_a_stream` reads the
//! sources and fails if one appears. That check is a source scan rather than a
//! test of behaviour because the thing being ruled out is an *absence*: a
//! stray print is invisible until the day somebody pipes the output into
//! `jq`, and by then it is in a release.

use std::io::{self, Write};

use clap::ValueEnum;

use crate::report::Report;

/// Which of the two output paths a command takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Prose, for a person reading a terminal.
    Human,
    /// One JSON object on one line, for a program reading a pipe.
    Json,
}

impl Format {
    /// Write `report` to the streams this format owns.
    ///
    /// # Errors
    ///
    /// Any failure from the underlying write. A closed pipe is the common one:
    /// `sure history | head -1` closes stdout as soon as the reader has what it
    /// wanted. The caller decides what that means; it is not swallowed here,
    /// because a report that did not reach the user is not a report that was
    /// delivered.
    pub fn emit(self, report: &Report) -> io::Result<()> {
        match self {
            Self::Json => {
                let stdout = io::stdout();
                let mut out = stdout.lock();
                report.machine(&mut out)?;
                writeln!(out)
            }
            Self::Human => {
                // An answer belongs on stdout; a complaint belongs on stderr.
                // `Report::is_an_answer` is the whole of that decision, and it
                // is made once rather than at each call site.
                if report.is_an_answer() {
                    let stdout = io::stdout();
                    report.human(&mut stdout.lock())
                } else {
                    let stderr = io::stderr();
                    report.human(&mut stderr.lock())
                }
            }
        }
    }
}

/// Report on stderr that a report could not be written.
///
/// The one write in this crate that is not a report. It exists so that a
/// failure to write does not become silence: a command whose answer never
/// reached the user must not exit as though it had.
pub fn write_stream_failure(error: &io::Error) {
    let stderr = io::stderr();
    let mut out = stderr.lock();
    let _ = writeln!(
        out,
        "sure: the result could not be written to the output stream: {error}"
    );
    let _ = writeln!(
        out,
        "The command ran. Its result did not reach you, so SURE is not reporting it as done."
    );
}

//! `sure doctor`, in the two forms a result can take.
//!
//! The facts are `sure_core::doctor`'s. This module decides what a person reads
//! and what a script reads, and the two are written independently — neither is a
//! formatting of the other — for the reason in `crate::report`'s module
//! documentation.
//!
//! # The wording rule
//!
//! `docs/product/UX_AND_LANGUAGE.md`: consequence first, no jargon. "journal
//! mode wal" appears because a user pasting this into a bug report should not
//! have to translate it back, and it is under a heading that says what it is.
//!
//! # What is deliberately not printed
//!
//! Nothing from the settings file, and no record's contents. The report has no
//! field that could hold either, which is what makes that a property of the
//! design rather than of this function's self-control.

use std::io::{self, Write};

use serde_json::{Value, json};
use sure_core::doctor::{DoctorReport, Places, Presence, Problem, StoreState, Tool};

/// The column a value begins in: two spaces of indent, the label, then a gap.
///
/// Fixed rather than computed from the longest label, so that two runs on two
/// machines line up and a diff between two pasted reports shows only what
/// changed. The longest label here is "evidence and history", twenty characters,
/// so the gap is what is left over.
const COLUMN: usize = 23;

/// The width the label is padded to, so that [`COLUMN`] is where its value
/// starts.
///
/// One less than the span from the indent to the value column, because the
/// space between them is a real space rather than part of the padding: padding
/// to the full width put "evidence and history" and `C:\Users\…` against each
/// other as `historyC:\Users\…`. A label longer than this is not truncated and
/// not wrapped — the line simply runs long, which is visible, whereas a glued
/// pair of columns reads as one value.
const LABEL: usize = COLUMN - 3;

/// Write the human form.
///
/// # Errors
///
/// Any failure from `out`.
pub fn human(report: &DoctorReport, out: &mut impl Write) -> io::Result<()> {
    let build = &report.build;
    writeln!(
        out,
        "SURE {} (harness protocol {}), built for {} {}",
        build.version, build.protocol_version, build.os, build.arch
    )?;
    if let Some(from) = &build.running_from {
        writeln!(out, "running from {}", from.display())?;
    }
    writeln!(out)?;

    places_in_words(&report.places, out)?;

    writeln!(out, "What SURE found there")?;
    match &report.places {
        Places::Unknown { .. } => {
            row(out, "everything", "nowhere to look")?;
        }
        Places::Known(locations) => {
            row(
                out,
                "evidence and history",
                &presence_in_words(locations.data_dir.presence.clone()),
            )?;
            row(
                out,
                "settings",
                &presence_in_words(locations.config_dir.presence.clone()),
            )?;
            row(
                out,
                "settings file",
                &presence_in_words(locations.settings_file.presence.clone()),
            )?;
            row(
                out,
                "record store",
                &store_in_words(&report.store, &locations.store_file.presence),
            )?;
        }
    }
    writeln!(out)?;

    writeln!(out, "Programs SURE calls")?;
    for tool in &report.tools {
        match &tool.found_at {
            Some(path) => row(out, tool.name, &format!("found at {}", path.display()))?,
            None => row(out, tool.name, "not found on PATH")?,
        }
        // The reason is on its own line because it is a sentence: putting it in
        // the value column would push every later line across the terminal.
        writeln!(out, "{:COLUMN$}{}", "", tool.needed_for)?;
    }
    writeln!(out)?;

    if report.problems.is_empty() {
        writeln!(out, "SURE found nothing wrong with its own files.")?;
    } else {
        writeln!(out, "What is wrong")?;
        for problem in &report.problems {
            writeln!(out, "  {}", problem.what)?;
            writeln!(out, "      {}", problem.detail)?;
        }
        writeln!(out)?;
        writeln!(
            out,
            "SURE exited with status {} rather than {}, because a script checking this \
             machine has to be able to tell it apart from one that is fine.",
            crate::report::exit::NOT_GREEN,
            crate::report::exit::OK
        )?;
    }
    writeln!(out)?;

    writeln!(out, "What this did not check")?;
    for entry in &report.not_checked {
        writeln!(out, "  {}", entry.what)?;
        writeln!(out, "      {}", entry.why)?;
    }
    writeln!(out)?;

    writeln!(
        out,
        "Settings are read by `sure config show`. Anything SURE has recorded is deleted with \
         `sure history delete`."
    )
}

/// The four locations, with the heading that says what they are.
fn places_in_words(places: &Places, out: &mut impl Write) -> io::Result<()> {
    match places {
        Places::Known(locations) => {
            writeln!(out, "Where SURE keeps things on this machine")?;
            row(
                out,
                "evidence and history",
                &locations.data_dir.path.display().to_string(),
            )?;
            row(
                out,
                "settings",
                &locations.config_dir.path.display().to_string(),
            )?;
            row(
                out,
                "settings file",
                &locations.settings_file.path.display().to_string(),
            )?;
            row(
                out,
                "record store",
                &locations.store_file.path.display().to_string(),
            )?;
        }
        Places::Unknown { what, detail } => {
            // The one thing this command must never do is print a path it
            // guessed. `docs/architecture/STORAGE_AND_DATA_PATHS.md`: the
            // nearest guess is the project being checked.
            writeln!(out, "SURE does not know where to keep its files here")?;
            writeln!(out, "It could not find {what}.")?;
            writeln!(out)?;
            writeln!(out, "{detail}")?;
        }
    }
    writeln!(out)
}

/// A label and its value, in the column layout.
fn row(out: &mut impl Write, label: &str, value: &str) -> io::Result<()> {
    writeln!(out, "  {label:<LABEL$} {value}")
}

/// What was found at a path, in the words a person uses.
fn presence_in_words(presence: Presence) -> String {
    match presence {
        Presence::Present => "there".to_owned(),
        Presence::Absent => "not created yet".to_owned(),
        Presence::Unreadable { detail } => {
            format!("there, and SURE could not look at it ({detail})")
        }
    }
}

/// What the store holds, in the words a person uses.
fn store_in_words(state: &StoreState, file: &Presence) -> String {
    match state {
        StoreState::NotCreated => "not created yet, so nothing is recorded".to_owned(),
        StoreState::NotLookedFor => "not looked at".to_owned(),
        StoreState::Unreadable { .. } => match file {
            Presence::Unreadable { .. } => "there, and SURE could not look at it".to_owned(),
            _ => "there, and SURE could not read it".to_owned(),
        },
        StoreState::Open(facts) => format!(
            "{} record{}, schema {}, journal mode {}",
            facts.records,
            if facts.records == 1 { "" } else { "s" },
            facts.schema_version,
            facts.journal_mode
        ),
    }
}

/// The `details` object of the response frame.
///
/// One object holding everything this command found, so that the frame's four
/// fixed fields keep meaning what `crate::report` says they mean and a reader
/// switches on `outcome` before looking in here.
#[must_use]
pub fn machine(report: &DoctorReport) -> Value {
    json!({
        "build": {
            "version": report.build.version,
            "protocol_version": report.build.protocol_version,
            "os": report.build.os,
            "arch": report.build.arch,
            "running_from": report.build.running_from.as_ref().map(|path| path.display().to_string()),
        },
        "places": places_machine(&report.places),
        "store": store_machine(&report.store),
        "tools": report.tools.iter().map(tool_machine).collect::<Vec<Value>>(),
        "problems": report.problems.iter().map(problem_machine).collect::<Vec<Value>>(),
        "not_checked": report.not_checked.iter().map(|entry| json!({
            "what": entry.what,
            "why": entry.why,
        })).collect::<Vec<Value>>(),
    })
}

fn places_machine(places: &Places) -> Value {
    match places {
        Places::Known(locations) => json!({
            "state": "known",
            "data_dir": place_machine(&locations.data_dir.path, &locations.data_dir.presence),
            "config_dir": place_machine(&locations.config_dir.path, &locations.config_dir.presence),
            "settings_file": place_machine(&locations.settings_file.path, &locations.settings_file.presence),
            "store_file": place_machine(&locations.store_file.path, &locations.store_file.presence),
        }),
        Places::Unknown { what, detail } => json!({
            "state": "unknown",
            "what": what,
            "detail": detail,
        }),
    }
}

fn place_machine(path: &std::path::Path, presence: &Presence) -> Value {
    json!({
        "path": path.display().to_string(),
        "presence": presence_name(presence),
    })
}

fn presence_name(presence: &Presence) -> &'static str {
    match presence {
        Presence::Present => "present",
        Presence::Absent => "absent",
        Presence::Unreadable { .. } => "unreadable",
    }
}

fn store_machine(state: &StoreState) -> Value {
    match state {
        StoreState::NotCreated => json!({"state": "not_created"}),
        StoreState::NotLookedFor => json!({"state": "not_looked_for"}),
        StoreState::Unreadable { detail } => json!({"state": "unreadable", "detail": detail}),
        StoreState::Open(facts) => json!({
            "state": "open",
            "journal_mode": facts.journal_mode,
            "schema_version": facts.schema_version,
            "records": facts.records,
        }),
    }
}

fn tool_machine(tool: &Tool) -> Value {
    json!({
        "name": tool.name,
        "needed_for": tool.needed_for,
        "found_at": tool.found_at.as_ref().map(|path| path.display().to_string()),
    })
}

fn problem_machine(problem: &Problem) -> Value {
    json!({
        "what": problem.what,
        "detail": problem.detail,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_core::doctor::{Build, DoctorReport, Locations, Place, StoreFacts};

    fn a_place(path: &str, presence: Presence) -> Place {
        Place {
            path: std::path::PathBuf::from(path),
            presence,
        }
    }

    /// A report of an installation that works, on a root this test names.
    fn a_healthy_report() -> DoctorReport {
        DoctorReport {
            build: Build {
                version: "0.0.0-test".to_owned(),
                protocol_version: 1,
                os: "windows",
                arch: "x86_64",
                running_from: Some(std::path::PathBuf::from(r"C:\tools\sure.exe")),
            },
            places: Places::Known(Box::new(Locations {
                data_dir: a_place(r"C:\Users\a\AppData\Local\SURE", Presence::Present),
                config_dir: a_place(r"C:\Users\a\AppData\Roaming\SURE", Presence::Absent),
                settings_file: a_place(
                    r"C:\Users\a\AppData\Roaming\SURE\sure.yaml",
                    Presence::Absent,
                ),
                store_file: a_place(r"C:\Users\a\AppData\Local\SURE\sure.db", Presence::Present),
            })),
            store: StoreState::Open(StoreFacts {
                journal_mode: "wal".to_owned(),
                schema_version: 4,
                records: 12,
            }),
            tools: vec![
                Tool {
                    name: "git",
                    needed_for: "identifying which state of a project a result belongs to",
                    found_at: Some(std::path::PathBuf::from(
                        r"C:\Program Files\Git\cmd\git.exe",
                    )),
                },
                Tool {
                    name: "not-installed",
                    needed_for: "something this test made up",
                    found_at: None,
                },
            ],
            problems: Vec::new(),
            not_checked: vec![sure_core::doctor::NotChecked {
                what: "whether a program found on PATH runs",
                why: "SURE has no process runner yet.",
            }],
        }
    }

    fn rendered(report: &DoctorReport) -> String {
        let mut buffer = Vec::new();
        human(report, &mut buffer).expect("the human form writes");
        String::from_utf8(buffer).expect("the human form is utf-8")
    }

    #[test]
    fn every_label_the_report_prints_is_separated_from_its_value() {
        // The regression this exists for: the value column was first computed so
        // that the longest label filled it exactly, and the two ran together as
        // `evidence and historyC:\Users\…`. A label and the thing it labels are
        // two pieces of text, and no reader should have to work out where one
        // stops.
        //
        // Every label the renderer can print is named here, including the
        // longest — if a label outgrows the column this fails on the row it
        // outgrew, rather than on whichever row a scan happened to reach.
        const LABELS: &[&str] = &[
            "evidence and history",
            "settings",
            "settings file",
            "record store",
            "git",
            "not-installed",
        ];
        let text = rendered(&a_healthy_report());

        for label in LABELS {
            let line = text
                .lines()
                .find(|line| label_of(line).trim_end() == *label)
                .unwrap_or_else(|| panic!("no row is labelled {label:?}:\n{text}"));
            assert_eq!(
                label_of(line).chars().count(),
                LABEL,
                "the padding for {label:?} is not the column width, so the values below it \
                 do not line up: {line:?}"
            );
            assert_eq!(
                line.chars().nth(COLUMN - 1),
                Some(' '),
                "the value is glued to {label:?}: {line:?}"
            );
            assert_ne!(
                line.chars().nth(COLUMN),
                Some(' '),
                "there is no value after {label:?}: {line:?}"
            );
        }
    }

    /// The label field of a line: the two spaces of indent, then up to the value
    /// column.
    fn label_of(line: &str) -> String {
        line.chars().skip(2).take(LABEL).collect()
    }

    #[test]
    fn what_was_found_wrong_is_never_left_out() {
        // The false-green rule in its last line of defence. A report carrying a
        // problem must say so on the terminal, and a report carrying none must
        // not say it found something — the two sentences are exclusive.
        let healthy = rendered(&a_healthy_report());
        assert!(healthy.contains("SURE found nothing wrong"), "{healthy}");
        assert!(!healthy.contains("What is wrong"), "{healthy}");

        let mut damaged = a_healthy_report();
        damaged.store = StoreState::Unreadable {
            detail: "the file is not a database".to_owned(),
        };
        damaged.problems.push(Problem {
            what: "SURE has recorded history on this machine and cannot read it.",
            detail: "the file is not a database".to_owned(),
        });
        let text = rendered(&damaged);

        assert!(text.contains("What is wrong"), "{text}");
        assert!(!text.contains("SURE found nothing wrong"), "{text}");
        assert!(
            text.contains("the file is not a database"),
            "the report does not say what was wrong, only that something was:\n{text}"
        );
        assert!(
            text.contains("exited with status 1"),
            "the human form does not say what the status will be, so a reader piping it \
             somewhere cannot tell:\n{text}"
        );
    }

    #[test]
    fn what_the_report_did_not_look_at_is_always_printed() {
        // The honesty rule: a report that stops at what it found invites the
        // reader to assume it looked everywhere.
        let text = rendered(&a_healthy_report());
        assert!(text.contains("What this did not check"), "{text}");
        assert!(
            text.contains("whether a program found on PATH runs"),
            "{text}"
        );
        assert!(
            !a_healthy_report().not_checked.is_empty(),
            "the fixture cannot demonstrate this with an empty list"
        );
    }

    #[test]
    fn a_tool_that_was_not_found_says_so_rather_than_showing_a_blank() {
        let text = rendered(&a_healthy_report());
        assert!(text.contains("not found on PATH"), "{text}");
        assert!(text.contains("found at"), "{text}");
    }

    #[test]
    fn the_machine_form_puts_what_was_found_under_one_key() {
        // A script switches on the frame's outcome before it looks anywhere
        // else, so everything a doctor run found lives under one key rather than
        // being spread across the frame's own fields.
        let value = machine(&a_healthy_report());

        assert_eq!(value["places"]["state"], "known");
        assert_eq!(value["store"]["state"], "open");
        assert_eq!(value["store"]["journal_mode"], "wal");
        assert_eq!(value["store"]["records"], 12);
        assert!(value["problems"].as_array().expect("a list").is_empty());
        assert_eq!(
            value["tools"][1]["found_at"],
            Value::Null,
            "a tool that was not found is a null rather than a missing key"
        );
        assert_eq!(
            value["not_checked"][0]["what"],
            "whether a program found on PATH runs"
        );
        assert_eq!(value["build"]["version"], "0.0.0-test");
    }
}

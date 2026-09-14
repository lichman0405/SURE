//! What SURE says about its own work.
//!
//! Two things are recorded about a SURE run: what it found out about the
//! project, and what SURE itself did. This module is the second one. The
//! distinction is not cosmetic — `docs/architecture/EVIDENCE_MODEL.md` puts
//! SURE's own activity at the bottom of the truth order, and a diagnostic is
//! **never** evidence about the project. A line saying that a check was planned
//! is not a line saying it passed. Nothing here is allowed to become a
//! `CheckResult`, and the types are separate so that it cannot happen by
//! accident.
//!
//! Diagnostics are also not the report. The report is user-facing prose written
//! to the rules in `docs/product/UX_AND_LANGUAGE.md`; these lines are for a
//! user running `sure doctor`, for a bug report, and for SURE's own
//! maintainers. They are terse, lowercase and machine-readable on purpose, and
//! nothing here should be pasted into a verdict.
//!
//! # What a line is
//!
//! ```text
//! 2026-09-14T09:10:56.827Z warn  run_01j2m… session=ses_… check=chk_…  config was not read  path="sure.yaml" reason="it is a directory"
//! ```
//!
//! Fixed parts: a timestamp, a level, a correlation, a message. Values go in
//! [`Field`]s, which redact themselves.
//!
//! # Why the message is a literal
//!
//! [`Diagnostic`]'s message is `&'static str`. That is deliberate and it is the
//! same rule as [`sure_domain::status::NO_TRUSTED_INTENT_LIMITATION`]: a
//! sentence SURE says is a constant, so no call site can paraphrase it into
//! something weaker, and no project-controlled string can be interpolated into
//! it. Everything variable goes in a field, where it is quoted, escaped and
//! redacted.
//!
//! # No secret is required
//!
//! `docs/security/SECRET_REDACTION.md` asks that a secret never have to be
//! handled to be reported. Three things make that true here:
//!
//! 1. [`Field::redacted`] records that a credential was present and takes no
//!    value, so the code path that knows about a secret does not have to pass
//!    one along to say so.
//! 2. [`Field::text`] discards a value whose *key* names a credential, without
//!    inspecting it.
//! 3. Every other value passes through [`crate::redact`] before it is stored.
//!
//! None of those claim to detect every secret, which is why
//! `docs/security/SECRET_REDACTION.md` is explicit that detection is imperfect
//! and this module does not repeat the claim that it is.
//!
//! # Where lines go
//!
//! To a `Write` chosen by the caller — stderr, or a local log file. There is
//! deliberately no constructor that writes to stdout: `sure hook ingest` must
//! emit only the harness response contract on stdout, and debug noise there
//! would corrupt it (`docs/architecture/EVENT_PROTOCOL.md`). Making stdout
//! unreachable by construction is worth more than a rule in a comment.

mod correlation;
mod field;
mod time;

pub use correlation::Correlation;
pub use field::{Field, FieldValue, NOT_RECORDED};
pub use time::Timestamp;

use std::io::{self, Write};

use sure_domain::ids::CheckId;

/// How serious a line is.
///
/// Ordered from most to least serious, so a threshold reads as "this level and
/// above". There is no `Fatal`: a diagnostic is never the reason SURE stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Something SURE could not do. The run may still produce a verdict.
    Error,
    /// Something unexpected that did not stop SURE, or a check that could not
    /// run.
    Warn,
    /// A step that completed, at the level a user would want to see.
    Info,
    /// Detail for diagnosing a problem.
    Debug,
    /// Everything, including values that are noisy but occasionally decisive.
    Trace,
}

impl Level {
    /// The name written into a line.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    /// The level a caller gets from a command-line verbosity count.
    ///
    /// `0` is [`Level::Warn`] rather than [`Level::Info`]: a check that runs
    /// for a minute and prints a line per step trains people to ignore the
    /// output, and the lines that matter are the ones that are not routine.
    #[must_use]
    pub const fn from_verbosity(verbosity: u8) -> Self {
        match verbosity {
            0 => Self::Warn,
            1 => Self::Info,
            2 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One recorded line.
///
/// Built by hand in tests and by [`Recorder::record`] in the product. Kept
/// separate from the recorder so that the format can be asserted without a
/// sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    level: Level,
    at: Timestamp,
    correlation: Correlation,
    message: &'static str,
    fields: Vec<Field>,
}

impl Diagnostic {
    /// A line about one instant, correlated to one run.
    #[must_use]
    pub fn new(
        level: Level,
        at: Timestamp,
        correlation: Correlation,
        message: &'static str,
    ) -> Self {
        Self {
            level,
            at,
            correlation,
            message,
            fields: Vec::new(),
        }
    }

    /// The same, with one more value.
    #[must_use]
    pub fn with_field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    /// The same, with several more values.
    #[must_use]
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = Field>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// How serious the line is.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// When it happened.
    #[must_use]
    pub const fn at(&self) -> Timestamp {
        self.at
    }

    /// What it is about.
    #[must_use]
    pub const fn correlation(&self) -> &Correlation {
        &self.correlation
    }

    /// The fixed sentence, unredacted and unescaped.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message
    }

    /// The values attached to it.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// The line as text: one line, always.
    ///
    /// The message and every field are reduced to a single physical line. A
    /// value cannot introduce one, which is the property that lets a log file
    /// be parsed by splitting on `\n`.
    #[must_use]
    pub fn to_text_line(&self) -> String {
        let mut line = format!(
            "{} {:<5} {}  {}",
            self.at,
            self.level.as_str(),
            self.correlation,
            self.message
        );
        for field in &self.fields {
            line.push(' ');
            line.push_str(&field.to_string());
        }
        line
    }
}

/// The record as a line of text.
///
/// Nothing here redacts. Every value in a [`Diagnostic`] is either a fixed part
/// SURE wrote itself — the level, the correlation, the message — or a [`Field`],
/// which redacted itself when it was built. There is no way for a caller's
/// `String` to reach this type without passing through one.
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_text_line())
    }
}

/// Where diagnostics are written, and what is written.
///
/// Holds the correlation for a run so that callers do not repeat it on every
/// line, and a level threshold so that a normal run is quiet. The threshold is
/// a deliberate filter, not silent loss: a line below it was asked for by
/// nobody.
pub struct Recorder {
    correlation: Correlation,
    threshold: Level,
    out: Box<dyn Write + Send>,
}

impl Recorder {
    /// A recorder writing to the given sink.
    ///
    /// There is no `to_stdout`. See the module documentation: stdout carries
    /// the harness response contract and must not carry anything else.
    #[must_use]
    pub fn to_writer(
        out: impl Write + Send + 'static,
        correlation: Correlation,
        threshold: Level,
    ) -> Self {
        Self {
            correlation,
            threshold,
            out: Box::new(out),
        }
    }

    /// A recorder writing to standard error, which is where a command-line
    /// tool's diagnostics belong.
    #[must_use]
    pub fn to_stderr(correlation: Correlation, threshold: Level) -> Self {
        Self::to_writer(io::stderr(), correlation, threshold)
    }

    /// What this recorder's lines are correlated to.
    #[must_use]
    pub const fn correlation(&self) -> &Correlation {
        &self.correlation
    }

    /// A correlation narrowed to one check, carrying this run and session.
    #[must_use]
    pub fn for_check(&self, check: CheckId) -> Correlation {
        self.correlation.for_check(check)
    }

    /// Whether a level would be recorded.
    ///
    /// `Error` is the smallest variant, so "at least this serious" is `<=` in
    /// the ordering used here. Not `const`: comparing entails a trait call.
    #[must_use]
    pub fn records(&self, level: Level) -> bool {
        level <= self.threshold
    }

    /// Write one line.
    ///
    /// # Errors
    ///
    /// Returns the sink's error. This does not swallow it: a caller that
    /// ignores the failure is visibly ignoring it, and the failure is never
    /// mistaken for "there was nothing to report". What a caller does about it
    /// is the caller's decision — a diagnostic that cannot be written is not a
    /// reason to abandon a check.
    pub fn record(
        &mut self,
        level: Level,
        message: &'static str,
        fields: &[Field],
    ) -> io::Result<()> {
        if !self.records(level) {
            return Ok(());
        }
        let diagnostic =
            Diagnostic::new(level, Timestamp::now(), self.correlation.clone(), message)
                .with_fields(fields.iter().cloned());
        writeln!(self.out, "{}", diagnostic.to_text_line())
    }

    /// Write one line that has already been built, for a caller that needed a
    /// timestamp or a correlation other than this recorder's.
    ///
    /// # Errors
    ///
    /// As [`Recorder::record`]. The threshold is not applied: a caller that
    /// built the line itself has already decided to write it.
    pub fn write(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        writeln!(self.out, "{}", diagnostic.to_text_line())
    }

    /// Flush the sink.
    ///
    /// # Errors
    ///
    /// Returns the sink's error. A short-lived process — which is most of
    /// SURE's processes, since there is no daemon — should call this before it
    /// exits rather than rely on a buffered writer being dropped.
    pub fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::ids::{CheckId, RunId, SessionId};

    /// Records lines into a buffer that the test can read. `Arc<Mutex<…>>`
    /// because the recorder owns its sink, and a `&mut` borrow would outlive
    /// the test's ability to look.
    #[derive(Clone, Default)]
    struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Shared {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
        fn lines(&self) -> Vec<String> {
            self.text().lines().map(str::to_owned).collect()
        }
    }

    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    fn diagnostic() -> Diagnostic {
        Diagnostic::new(
            Level::Warn,
            at(1_700_000_000_000),
            Correlation::run(RunId::generate()),
            "config was not read",
        )
    }

    #[test]
    fn a_line_carries_the_time_the_level_the_run_and_the_message() {
        let run = RunId::generate();
        let line = Diagnostic::new(
            Level::Warn,
            at(1_700_000_000_000),
            Correlation::run(run.clone()),
            "config was not read",
        )
        .to_text_line();

        assert!(line.starts_with("2023-11-14T22:13:20.000Z "), "{line}");
        assert!(line.contains(" warn "), "{line}");
        assert!(line.contains(run.as_str()), "{line}");
        assert!(line.ends_with("config was not read"), "{line}");
    }

    #[test]
    fn the_session_and_check_appear_when_they_are_known() {
        let session = SessionId::generate();
        let check = CheckId::generate();
        let line = Diagnostic::new(
            Level::Info,
            at(0),
            Correlation::run(RunId::generate())
                .with_session(session.clone())
                .with_check(check.clone()),
            "check started",
        )
        .to_text_line();
        assert!(line.contains(&format!("session={session}")), "{line}");
        assert!(line.contains(&format!("check={check}")), "{line}");
    }

    #[test]
    fn fields_are_appended_after_the_message() {
        let line = diagnostic()
            .with_field(Field::text("path", "sure.yaml"))
            .with_field(Field::number("line", 4))
            .with_field(Field::boolean("readable", false))
            .to_text_line();
        assert!(
            line.ends_with("path=\"sure.yaml\" line=4 readable=false"),
            "{line}"
        );
    }

    #[test]
    fn a_line_is_exactly_one_line_whatever_the_values_contain() {
        // What a log reader depends on: a value cannot become a record, and a
        // record cannot become two.
        let shared = Shared::default();
        let mut recorder = Recorder::to_writer(
            shared.clone(),
            Correlation::run(RunId::generate()),
            Level::Trace,
        );
        for message in [
            "config was not read",
            "a check could not run",
            "the run finished",
        ] {
            recorder
                .record(
                    Level::Warn,
                    message,
                    &[Field::text(
                        "reason",
                        "line one\nline two\r\nSURE: all good",
                    )],
                )
                .unwrap();
        }
        assert_eq!(shared.lines().len(), 3, "{}", shared.text());
        assert_eq!(shared.text().matches('\n').count(), 3);
    }

    #[test]
    fn a_secret_passed_as_a_field_value_does_not_reach_the_sink() {
        // The end-to-end version of the field-level test: through a real
        // recorder, into a real buffer, with a credential that a caller really
        // did hand over.
        let shared = Shared::default();
        let mut recorder = Recorder::to_writer(
            shared.clone(),
            Correlation::run(RunId::generate()),
            Level::Trace,
        );
        recorder
            .record(
                Level::Warn,
                "the provider was rejected",
                &[
                    Field::text("api_key", "sk-abcdefghijklmnopqrstuvwxyz01"),
                    Field::text("endpoint", "https://alice:hunter2@api.example.com"),
                    Field::redacted("provider_credential"),
                ],
            )
            .unwrap();

        let text = shared.text();
        assert!(!text.contains("sk-abcdefghijklmnopqrstuvwxyz01"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");
        assert!(text.contains("api_key=<not recorded>"), "{text}");
        assert!(
            text.contains("provider_credential=<not recorded>"),
            "{text}"
        );
    }

    #[test]
    fn the_threshold_drops_quieter_lines_and_keeps_the_rest() {
        let shared = Shared::default();
        let mut recorder = Recorder::to_writer(
            shared.clone(),
            Correlation::run(RunId::generate()),
            Level::Warn,
        );
        for (level, message) in [
            (Level::Error, "the run failed"),
            (Level::Warn, "a check did not run"),
            (Level::Info, "the run started"),
            (Level::Debug, "the plan was built"),
        ] {
            recorder.record(level, message, &[]).unwrap();
        }
        let text = shared.text();
        assert!(text.contains("the run failed"), "{text}");
        assert!(text.contains("a check did not run"), "{text}");
        assert!(!text.contains("the run started"), "{text}");
        assert!(!text.contains("the plan was built"), "{text}");
        assert!(!recorder.records(Level::Info));
        assert!(recorder.records(Level::Error));
    }

    #[test]
    fn verbosity_maps_to_a_threshold_that_stays_quiet_by_default() {
        assert_eq!(Level::from_verbosity(0), Level::Warn);
        assert_eq!(Level::from_verbosity(1), Level::Info);
        assert_eq!(Level::from_verbosity(2), Level::Debug);
        assert_eq!(Level::from_verbosity(9), Level::Trace);
    }

    #[test]
    fn a_check_scoped_correlation_keeps_the_run_the_recorder_started_with() {
        let run = RunId::generate();
        let recorder = Recorder::to_writer(
            Shared::default(),
            Correlation::run(run.clone()),
            Level::Trace,
        );
        let check = CheckId::generate();
        let scoped = recorder.for_check(check.clone());

        assert_eq!(scoped.run_id(), &run);
        assert_eq!(scoped.check_id(), Some(&check));
        assert_eq!(
            recorder.correlation().check_id(),
            None,
            "the recorder changed"
        );
    }

    #[test]
    fn a_sink_that_fails_reports_the_failure_rather_than_hiding_it() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("the disk is full"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut recorder =
            Recorder::to_writer(Broken, Correlation::run(RunId::generate()), Level::Trace);
        let error = recorder
            .record(Level::Error, "the run failed", &[])
            .unwrap_err();
        assert_eq!(error.to_string(), "the disk is full");
    }

    #[test]
    fn the_level_names_are_the_ones_written_into_a_line() {
        for (level, name) in [
            (Level::Error, "error"),
            (Level::Warn, "warn"),
            (Level::Info, "info"),
            (Level::Debug, "debug"),
            (Level::Trace, "trace"),
        ] {
            assert_eq!(level.as_str(), name);
            assert_eq!(level.to_string(), name);
        }
        // Ordered most serious first, which is what makes the threshold a `<=`.
        assert!(Level::Error < Level::Warn);
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Debug < Level::Trace);
    }
}

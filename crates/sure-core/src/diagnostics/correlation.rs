//! What a diagnostic line is about.
//!
//! Three identities, in decreasing scope:
//!
//! | Field | Always present | Identifies |
//! | --- | --- | --- |
//! | `run` | yes | one execution of SURE |
//! | `session` | no | the harness session SURE is observing |
//! | `check` | no | the check currently being evaluated |
//!
//! A run exists whether or not a harness is present, which is why it is the one
//! field that is never optional: `sure check` started by hand produces
//! diagnostics with a run, no session, and a check while checks are running. A
//! session is only known when SURE was started by an integration, and it is
//! absent rather than invented when it is not known
//! (`docs/architecture/EVENT_PROTOCOL.md`: "Missing data is not invented").
//!
//! Session and check are deliberately *not* alternatives to one another. A
//! check runs inside a session when there is one, and the two answer different
//! questions — "what was the agent doing" and "what is SURE doing now".

use std::fmt;

use sure_domain::ids::{CheckId, RunId, SessionId};

/// The identities a diagnostic is recorded against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correlation {
    run: RunId,
    session: Option<SessionId>,
    check: Option<CheckId>,
}

impl Correlation {
    /// The correlation for a run, with nothing narrower known yet.
    #[must_use]
    pub const fn run(run: RunId) -> Self {
        Self {
            run,
            session: None,
            check: None,
        }
    }

    /// The same, with the harness session that started it.
    #[must_use]
    pub fn with_session(mut self, session: SessionId) -> Self {
        self.session = Some(session);
        self
    }

    /// The same, narrowed to one check.
    #[must_use]
    pub fn with_check(mut self, check: CheckId) -> Self {
        self.check = Some(check);
        self
    }

    /// The same correlation narrowed to a check, keeping the run and session.
    ///
    /// This is the form the check runner uses: it holds one run correlation and
    /// derives a child per check, so every line inside a check carries the
    /// identities that lead back to the run that produced it.
    #[must_use]
    pub fn for_check(&self, check: CheckId) -> Self {
        Self {
            run: self.run.clone(),
            session: self.session.clone(),
            check: Some(check),
        }
    }

    /// The run this belongs to.
    #[must_use]
    pub const fn run_id(&self) -> &RunId {
        &self.run
    }

    /// The harness session, when there is one.
    #[must_use]
    pub const fn session_id(&self) -> Option<&SessionId> {
        self.session.as_ref()
    }

    /// The check currently being evaluated, when there is one.
    #[must_use]
    pub const fn check_id(&self) -> Option<&CheckId> {
        self.check.as_ref()
    }
}

impl fmt::Display for Correlation {
    /// The identities as `run_… session=… check=…`, omitting what is unknown.
    ///
    /// Omitted rather than written as a placeholder: a line that says
    /// `session=none` invites the reading that SURE looked and found nothing,
    /// when the truth is that SURE was not told.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.run)?;
        if let Some(session) = &self.session {
            write!(f, " session={session}")?;
        }
        if let Some(check) = &self.check {
            write!(f, " check={check}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_run_alone_is_a_complete_correlation() {
        let run = RunId::generate();
        let correlation = Correlation::run(run.clone());
        assert_eq!(correlation.run_id(), &run);
        assert!(correlation.session_id().is_none());
        assert!(correlation.check_id().is_none());
        assert_eq!(correlation.to_string(), run.as_str());
    }

    #[test]
    fn unknown_identities_are_omitted_rather_than_rendered_as_empty() {
        // `session=-` or `session=none` would read as "SURE checked and found
        // no session". It was not told, which is a different statement.
        let text = Correlation::run(RunId::generate()).to_string();
        assert!(!text.contains("session"), "{text}");
        assert!(!text.contains("check"), "{text}");
    }

    #[test]
    fn a_session_and_a_check_can_be_present_at_once() {
        let correlation = Correlation::run(RunId::generate())
            .with_session(SessionId::generate())
            .with_check(CheckId::generate());
        assert!(correlation.session_id().is_some());
        assert!(correlation.check_id().is_some());
        let text = correlation.to_string();
        assert!(text.contains("session=ses_"), "{text}");
        assert!(text.contains("check=chk_"), "{text}");
    }

    #[test]
    fn deriving_a_check_keeps_the_run_and_the_session() {
        // The property the check runner depends on: a line emitted inside a
        // check can be traced back to the run it belongs to.
        let run = RunId::generate();
        let session = SessionId::generate();
        let parent = Correlation::run(run.clone()).with_session(session.clone());
        let child = parent.for_check(CheckId::generate());

        assert_eq!(child.run_id(), &run);
        assert_eq!(child.session_id(), Some(&session));
        assert!(child.check_id().is_some());
        assert_eq!(parent.check_id(), None, "the parent was mutated");
    }

    #[test]
    fn two_checks_under_one_run_are_distinguishable() {
        let parent = Correlation::run(RunId::generate());
        let first = parent.for_check(CheckId::generate());
        let second = parent.for_check(CheckId::generate());
        assert_ne!(first, second);
        assert_eq!(first.run_id(), second.run_id());
    }
}

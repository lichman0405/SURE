//! The channels a statement about a project arrives on, and what each is worth.
//!
//! # The label belongs to the door
//!
//! Everything the product may say about a [`Requirement`] follows from one
//! field. [`Requirement::source`] decides whether SURE may compare a project
//! against the words at all, whether they count towards "everything you asked
//! for is done", and what a report is allowed to call them. A field a caller
//! fills in is a field a caller can fill in wrongly, and the mistake that
//! matters here is not a typo: it is a sentence read out of a README arriving as
//! something the user said.
//!
//! So every channel has a door, and each door writes its own label. **No
//! function here that produces a [`Requirement`] takes an [`IntentSource`]**, and
//! that is the property the doors exist for rather than a habit: a caller holding
//! a command it read out of a document has nothing to ask for, because no door
//! has a parameter to ask with. Provenance is preserved by a shape the module
//! does not offer an alternative to, rather than by a rule its code follows.
//!
//! One function here does take an [`IntentSource`], and it is worth naming rather
//! than leaving a reader to find it and doubt the rule. [`from_source`] goes the
//! other way: it reads statements *back* by the label they arrived with, and the
//! caller who can ask that question is the one the labels are for. The claim is
//! about the direction a label travels. A door that took one would be a caller
//! choosing what a statement is worth; a reader that takes one is asking what it
//! already was.
//!
//! # Where the doors are
//!
//! | Statement | Label | Door |
//! | --- | --- | --- |
//! | a goal typed to `sure check --goal` | `explicit_user_goal` | [`crate::project_intent::explicit_goal`] |
//! | a request a saved session exposed | `observed_user_request` | [`observed_user_request`] |
//! | a goal written into `sure.yaml` | `project_spec` | [`documented_goal`] |
//! | a fenced command in a document | `project_spec` | [`crate::documents::DocumentReport::as_requirements`] |
//! | an agent saying something is done | `agent_claim` | [`agent_claim`] |
//! | SURE's own guess | `inferred` | [`inferred`] |
//!
//! The two rows labelled `project_spec` are one channel reached by two doors,
//! and both ask [`Config::goal_source`](crate::config::Config::goal_source)
//! rather than naming the label themselves, so there is still one place in the
//! build that decides a project file is documentation.
//!
//! Three of those doors have nobody knocking on them yet, and it is worth being
//! exact about which rather than letting a table imply a pipeline this build
//! does not have. Nothing captures a harness session: that is `P8-T005`, which
//! depends on this module for the rule it has to satisfy. Nothing reads an
//! agent's completion claim, because a claim can only be found by reading a
//! sentence, and picking statements out of prose is the failure
//! `docs/security/THREAT_MODEL.md` files as T17. And nothing generates an
//! inference — this module is where a guess would be labelled, not where one is
//! made.
//!
//! # Why the observed door takes an [`Authority`] and not a [`Config`]
//!
//! `sure.yaml` lives inside the project being checked, and the project is
//! written by the same agent whose work is being judged, so
//! `privacy.full_recording: true` in that file is a **request**.
//! [`Config::requested_privileges`](crate::config::Config::requested_privileges)
//! lists it as one, and [`Authority::privilege`] answers whether anyone was able
//! to grant it. Within the configuration files, only the user's own settings
//! outside the project can — which is the whole of
//! `docs/architecture/CONFIG_AUTHORITY.md` in one predicate.
//!
//! A door that read a [`Config`] would therefore let a project open itself. This
//! one reads the resolved answer, so a session's words are kept when the person
//! at the keyboard allowed it, and not when the project asked to be allowed.
//!
//! # An inference is never a requirement
//!
//! [`inferred`] produces a statement SURE may report on and may never grade a
//! project against. That answer is [`IntentSource::is_user_requirement`]'s, and
//! it belongs in the domain rather than here for a reason worth stating: the
//! gate and the label must not be able to disagree.
//! [`sure_domain::intent::may_claim_full_fulfilment`] counts
//! [`ProjectIntent::user_requirements`], so an inference is not given a lower
//! rank — it is not in the list that gets counted.
//!
//! What this module adds is that no door produces a promoted one. Nothing here
//! takes a [`Requirement`] and hands back a [`Requirement`] with a different
//! source, so an inference stays an inference however an intent is put
//! together, and [`from_source`] reads statements back by the label they
//! arrived with.
//!
//! # What this module does not do
//!
//! It does not decide what a label is worth beyond naming it:
//! [`sure_domain::intent::RequirementAuthority`] derives that from the label, in
//! one place, and a second derivation here is how the two would come to
//! disagree. It does not compare a statement against a project — `P6-T008` owns
//! that. And it does not assemble a whole project's intent, because one of the
//! five channels is reached by reading a goal back out of the store, and
//! `docs/architecture/PROJECT_INTENT.md` records the gap that stands in the way
//! of a reader asking for one project's goal.

use std::fmt;

use sure_domain::intent::{IntentSource, ProjectIntent, Requirement};

use crate::config::{Authority, Config, ProjectRequest};

/// The identifier SURE gives a goal the project wrote down about itself.
///
/// Fixed rather than generated, for the reason
/// [`EXPLICIT_GOAL_ID`](crate::project_intent::EXPLICIT_GOAL_ID) gives for its
/// own: within one project there is one goal the project states about itself,
/// and a reader comparing this week's with last week's compares two strings
/// rather than matching identities. It is a different string from that one so
/// that a project which stated a goal on both channels has two statements
/// rather than one that overwrote the other.
pub const DOCUMENTED_GOAL_ID: &str = "documented-goal";

/// The identifier SURE gives the request a session exposed.
///
/// Fixed for the same reason as [`DOCUMENTED_GOAL_ID`], and it is the identifier
/// `P8-T005` is to write when it captures a session — the task that implements
/// the observed capture path depends on this module for that, and stating it
/// here is what makes the two agree without one of them copying the other.
pub const OBSERVED_REQUEST_ID: &str = "request";

/// Why a statement from a channel SURE can only reach under a permission was not
/// kept.
///
/// Two variants because the two failures have different remedies: one is a
/// setting the user can grant, and the other is words that are not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentRefusal {
    /// Full recording is not in effect, so SURE did not keep what a session
    /// exposed.
    ///
    /// This is the answer both when nobody asked for full recording and when the
    /// project asked and could not grant it. The two are different facts about
    /// the configuration — [`Authority::privileges`] keeps them apart and reports
    /// the refused escalation — but they are the same fact about this statement,
    /// which is that SURE did not keep it.
    RecordingNotPermitted,
    /// The statement had no words in it.
    ///
    /// A requirement with no words in it is not a requirement. Keeping one would
    /// put a blank row in a history that reads as something somebody asked for.
    NoWords,
}

impl fmt::Display for IntentRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordingNotPermitted => f.write_str(
                "SURE did not keep the request it saw, because full recording is not \
                 granted, so nothing was kept. A project file can ask for full \
                 recording; only your own SURE settings, outside the project, can allow \
                 it.",
            ),
            Self::NoWords => f.write_str(
                "SURE was given a statement with no words in it, so nothing was kept. A \
                 requirement is what a project is checked against, and an empty one would \
                 become a requirement no report could state.",
            ),
        }
    }
}

impl std::error::Error for IntentRefusal {}

/// The goal the project wrote down about itself, if it wrote one.
///
/// The words are kept exactly as the file had them, and the label is
/// [`Config::goal_source`], which is [`IntentSource::ProjectSpec`] — so this is
/// **documentation, and never the standard a project is graded against**.
/// `docs/architecture/CONFIG_REFERENCE.md` says both halves of that about the
/// setting, and `docs/adr/0011-project-configuration-is-a-request.md` says why:
/// the file is inside the project, and the project is written by the same agent
/// whose work is being judged.
///
/// `None` means the project documented no goal. A goal that is present but has
/// no words in it is the same answer rather than an error, because
/// [`Config::from_yaml`](crate::config::Config::from_yaml) refuses such a file
/// before a `Config` can hold one, and the only caller that can reach this with
/// one built it by hand. Manufacturing a requirement with no words in it would
/// be the worse answer to a case the reader already refuses.
#[must_use]
pub fn documented_goal(config: &Config) -> Option<Requirement> {
    let goal = config.project_intent.goal.as_deref()?;
    if goal.trim().is_empty() {
        return None;
    }
    Some(Requirement::new(DOCUMENTED_GOAL_ID, goal, Config::goal_source()).with_raw_retained(true))
}

/// The request a saved session exposed, if SURE was permitted to keep it.
///
/// The permission is checked before the words are read. SURE does not examine
/// the text of material it has no permission to keep, so a run without the
/// opt-in gets the same answer whether or not the session it saw had anything in
/// it — the refusal is about what SURE may hold, not about what it was shown.
///
/// The label is [`IntentSource::ObservedUserRequest`], which is one of the two
/// sources that can stand in for the user's goal, so a request kept here **is**
/// something SURE may compare a project against. The privacy decision is about
/// whether the words are kept, and it does not grade them: there is no
/// half-trusted version of this source, and inventing one would let a project be
/// judged against a requirement the report had already discounted.
///
/// # Errors
///
/// [`IntentRefusal::RecordingNotPermitted`] when
/// [`ProjectRequest::FullRecording`] was not granted, and
/// [`IntentRefusal::NoWords`] when there is nothing to keep.
pub fn observed_user_request(
    text: &str,
    authority: &Authority,
) -> Result<Requirement, IntentRefusal> {
    if !recording_is_granted(authority) {
        return Err(IntentRefusal::RecordingNotPermitted);
    }
    if text.trim().is_empty() {
        return Err(IntentRefusal::NoWords);
    }
    Ok(
        Requirement::new(OBSERVED_REQUEST_ID, text, IntentSource::ObservedUserRequest)
            .with_raw_retained(true),
    )
}

/// Something a coding agent said, which SURE may try to check and no more.
///
/// An agent's completion claim is evidence of what the agent asserted, never
/// proof that it was a requirement and never proof that it was done —
/// `docs/architecture/PROJECT_INTENT.md` says so under each of the three
/// headings it could be mistaken for.
///
/// [`raw_retained`](Requirement::raw_retained) is left false because this door
/// does not know whether the caller kept the agent's sentence or restated it.
/// A caller that quoted the sentence verbatim says so with
/// [`Requirement::with_raw_retained`]; the flag cannot be defaulted to true on
/// the door's behalf without the door knowing something it does not.
#[must_use]
pub fn agent_claim(id: impl Into<String>, text: impl Into<String>) -> Requirement {
    Requirement::new(id, text, IntentSource::AgentClaim)
}

/// Something SURE guessed about the project.
///
/// The guess is a [`Requirement`] so that it can be shown — a report that hid it
/// could not explain what it was about to compare — and it is
/// [`RequirementAuthority::NotARequirement`](sure_domain::intent::RequirementAuthority::NotARequirement),
/// so SURE may not report on it as a requirement at all.
///
/// [`raw_retained`](Requirement::raw_retained) is left false and there is
/// nothing for it to be true of: an inference has no original wording behind it,
/// because the words are SURE's own. The flag is about captured material, and
/// this statement was not captured from anywhere.
#[must_use]
pub fn inferred(id: impl Into<String>, text: impl Into<String>) -> Requirement {
    Requirement::new(id, text, IntentSource::Inferred)
}

/// Every statement in an intent that arrived on one channel.
///
/// A query rather than an accessor per variant, so that asking about a channel
/// is asking about the closed vocabulary. Five accessors would be a list
/// somebody has to remember to extend, and [`IntentSource::ALL`] is generated
/// from the vocabulary itself — so a source added to the domain is one this
/// function can answer for on the day it is added, rather than on the day
/// somebody notices.
///
/// No `#[must_use]`: the returned iterator already carries one, and repeating it
/// here would be an attribute that cannot fire.
pub fn from_source(
    intent: &ProjectIntent,
    source: IntentSource,
) -> impl Iterator<Item = &Requirement> {
    intent
        .requirements
        .iter()
        .filter(move |requirement| requirement.source == source)
}

/// Whether any layer was able to grant keeping the whole of a session.
///
/// [`Privilege::is_granted`](crate::config::Privilege::is_granted) is the whole
/// of the answer, and it is asked of the authority rather than of either file,
/// so a project that asks for more recording than the user allowed gets the
/// answer the user gave.
fn recording_is_granted(authority: &Authority) -> bool {
    authority
        .privilege(ProjectRequest::FullRecording)
        .is_some_and(|privilege| privilege.is_granted())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::{ConfigSource, Layer, LoadedConfig};

    /// Settings read from a file, the way a run reads them.
    ///
    /// Through [`Config::from_yaml`] rather than by assigning fields, so that a
    /// test cannot pass by setting a combination the reader would have refused.
    fn file(text: &str) -> LoadedConfig {
        LoadedConfig {
            config: Config::from_yaml(text).expect("the settings must parse"),
            source: ConfigSource::File(PathBuf::from("sure.yaml")),
            searched: PathBuf::from("sure.yaml"),
        }
    }

    /// A project with a file and no user settings, which is the ordinary case.
    fn project_only(text: &str) -> Authority {
        Authority::new(None, file(text))
    }

    /// A project file and a user file.
    fn both(user: &str, project: &str) -> Authority {
        Authority::new(Some(file(user)), file(project))
    }

    /// The one statement a single-requirement intent holds.
    fn only_requirement(intent: ProjectIntent) -> Requirement {
        let mut requirements = intent.requirements;
        assert_eq!(
            requirements.len(),
            1,
            "the door produced several statements"
        );
        requirements.remove(0)
    }

    #[test]
    fn a_documented_goal_is_documentation_and_says_so() {
        // What this checks is the *value* the door writes. It cannot check that
        // the door asks [`Config::goal_source`] instead of naming the same value
        // itself: both readings produce this assertion, and a test cannot move a
        // `const` to tell them apart. That the door asks is a decision, and
        // `target/tmp/mutate16.py` declares the mutation that removes it
        // unobservable rather than leaving a comment here claiming a check that
        // does not exist.
        let config = Config::from_yaml("project_intent:\n  goal: A local bookmark manager.\n")
            .expect("parse");

        let requirement = documented_goal(&config).expect("the project documented a goal");
        assert_eq!(requirement.id, DOCUMENTED_GOAL_ID);
        assert_eq!(requirement.text, "A local bookmark manager.");
        assert_eq!(requirement.source, IntentSource::ProjectSpec);
        assert_eq!(requirement.source, Config::goal_source());
        assert!(requirement.source.is_documentation());
        assert!(!requirement.is_user_requirement());
        assert!(
            requirement.raw_retained,
            "the file's words were kept as they were"
        );
    }

    #[test]
    fn each_channel_names_its_statement_something_else() {
        // The identifiers are fixed so that a reader comparing last week's
        // statement with this week's compares two strings, and they are
        // *different* from each other so that a project which stated its goal on
        // the project's own channel and again on the command line has two
        // statements rather than one that quietly took the other's place.
        //
        // The distinctness and not the spelling: what a document's goal is
        // called is a name nobody's behaviour depends on, which is why no test
        // here pins the literal and `mutate16.py` declares the spelling
        // unobservable.
        let ids = [
            crate::project_intent::EXPLICIT_GOAL_ID,
            DOCUMENTED_GOAL_ID,
            OBSERVED_REQUEST_ID,
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!id.trim().is_empty(), "{id:?} is not an identifier");
            assert_eq!(
                ids.iter().filter(|other| *other == id).count(),
                1,
                "{id:?} at position {index} is another channel's identifier"
            );
        }
    }

    #[test]
    fn the_projects_own_words_are_kept_as_the_file_had_them() {
        // The doc comment claims the words are kept exactly, and "exactly" is a
        // claim a door can lose by tidying up. Trimming is the tidy-up that looks
        // like care: the words still mean the same thing, and the statement is no
        // longer what the project wrote. It matters here because
        // `raw_retained: true` is a claim *about* the wording — a door that
        // trimmed and still said so would be reporting something it did not do.
        let config =
            Config::from_yaml("project_intent:\n  goal: \"  A local bookmark manager.  \"\n")
                .expect("parse");
        assert_eq!(
            documented_goal(&config).expect("documented").text,
            "  A local bookmark manager.  ",
            "the project's own spacing was tidied away"
        );
    }

    #[test]
    fn a_project_with_no_goal_documents_nothing() {
        assert!(documented_goal(&Config::default()).is_none());
        assert!(
            documented_goal(&Config::from_yaml("report:\n  format: json\n").expect("parse"))
                .is_none()
        );
    }

    #[test]
    fn a_goal_with_no_words_in_it_is_no_goal_rather_than_a_blank_requirement() {
        // Reachable only by a caller that built a `Config` by hand, because the
        // reader refuses such a file — `Config::validate_intent` says so. The
        // door still has to answer, and "no words" is the answer a goal with no
        // words in it gets everywhere else.
        for goal in ["", " ", "\t\n", "\u{00a0}"] {
            let mut config = Config::default();
            config.project_intent.goal = Some(goal.to_owned());
            assert!(
                documented_goal(&config).is_none(),
                "{goal:?} became a requirement"
            );
        }
    }

    #[test]
    fn a_request_kept_under_the_users_own_opt_in_is_a_user_requirement() {
        let authority = both("privacy:\n  full_recording: true\n", "");

        let requirement =
            observed_user_request("add CSV export", &authority).expect("the user allowed it");
        assert_eq!(requirement.id, OBSERVED_REQUEST_ID);
        assert_eq!(requirement.text, "add CSV export");
        assert_eq!(requirement.source, IntentSource::ObservedUserRequest);
        assert!(requirement.raw_retained);
        assert!(requirement.is_user_requirement());
        assert!(requirement.source.requires_full_recording());
        // Whole-trust or not at all. Keeping the words is the only thing the
        // permission decides, and a kept request counts exactly as much as a
        // goal that was typed: there is no half-trusted version of this source.
        assert_eq!(
            requirement.authority(),
            sure_domain::intent::RequirementAuthority::UserRequirement
        );
    }

    #[test]
    fn a_project_asking_for_full_recording_does_not_get_it() {
        // The escalation `docs/architecture/CONFIG_AUTHORITY.md` exists to
        // prevent, stopped at the door that would have acted on it.
        let authority = project_only("privacy:\n  full_recording: true\n");

        assert_eq!(
            observed_user_request("add CSV export", &authority),
            Err(IntentRefusal::RecordingNotPermitted)
        );
        let privilege = authority
            .privilege(ProjectRequest::FullRecording)
            .expect("the request was made and left a record");
        assert_eq!(privilege.asked_by, vec![Layer::Project]);
        assert!(privilege.is_refused_escalation());
    }

    #[test]
    fn a_run_that_asked_for_nothing_keeps_nothing() {
        let authority = project_only("");
        assert!(authority.privilege(ProjectRequest::FullRecording).is_none());
        assert_eq!(
            observed_user_request("add CSV export", &authority),
            Err(IntentRefusal::RecordingNotPermitted)
        );
    }

    #[test]
    fn the_permission_is_asked_before_the_words_are_read() {
        // The same refusal for a session with nothing in it and one with a
        // sentence in it, because the answer is about what SURE may hold rather
        // than about what it was shown. Checking emptiness first would make this
        // door report on the content of material it is not allowed to keep.
        let authority = project_only("");
        assert_eq!(
            observed_user_request("", &authority),
            Err(IntentRefusal::RecordingNotPermitted)
        );
        assert_eq!(
            observed_user_request("add CSV export", &authority),
            Err(IntentRefusal::RecordingNotPermitted)
        );
    }

    #[test]
    fn a_request_with_no_words_is_refused_once_the_permission_is_there() {
        let authority = both("privacy:\n  full_recording: true\n", "");
        for text in ["", " ", "\t\n"] {
            assert_eq!(
                observed_user_request(text, &authority),
                Err(IntentRefusal::NoWords),
                "{text:?} was kept as a request"
            );
        }
        // And the words that are kept are the words that were given. The padded
        // input is the point of asking here: a door that tidied the spacing away
        // would still be `Ok`, and `raw_retained: true` would then be a claim
        // about a wording SURE no longer holds.
        assert_eq!(
            observed_user_request("  add CSV export  ", &authority)
                .expect("the user allowed it")
                .text,
            "  add CSV export  ",
            "the request was edited on the way in"
        );
    }

    #[test]
    fn an_inference_has_no_raw_text_to_have_retained() {
        let requirement = inferred("guess-1", "this is probably a todo app");
        assert!(!requirement.raw_retained);
        assert!(!requirement.is_user_requirement());
        assert_eq!(
            requirement.authority(),
            sure_domain::intent::RequirementAuthority::NotARequirement
        );
    }

    #[test]
    fn a_claim_is_kept_without_the_door_deciding_whether_it_was_quoted() {
        let requirement = agent_claim("claim-1", "login is finished");
        assert!(!requirement.raw_retained);
        assert_eq!(requirement.source, IntentSource::AgentClaim);
        assert_eq!(
            requirement.authority(),
            sure_domain::intent::RequirementAuthority::AgentAssertion
        );
        // A caller that did quote the sentence can say so. The door leaving the
        // flag alone is not the door deciding it.
        assert!(
            agent_claim("claim-1", "login is finished")
                .with_raw_retained(true)
                .raw_retained
        );
    }

    #[test]
    fn every_door_writes_its_own_label_and_no_door_takes_one() {
        // The property the module is built on, checked over the whole closed
        // vocabulary rather than over the labels somebody remembered to list.
        // Each source has exactly one door, and the door produces that source
        // without being told it.
        let authority = both(
            "privacy:\n  full_recording: true\n",
            "project_intent:\n  goal: a bookmark manager\n",
        );

        let doors: Vec<(IntentSource, Requirement)> = vec![
            (
                IntentSource::ExplicitUserGoal,
                only_requirement(
                    crate::project_intent::explicit_goal("ship it").expect("a goal with words"),
                ),
            ),
            (
                IntentSource::ObservedUserRequest,
                observed_user_request("add CSV export", &authority).expect("granted"),
            ),
            (
                IntentSource::ProjectSpec,
                documented_goal(authority.project()).expect("documented"),
            ),
            (
                IntentSource::AgentClaim,
                agent_claim("claim-1", "login is finished"),
            ),
            (
                IntentSource::Inferred,
                inferred("guess-1", "probably a todo app"),
            ),
        ];

        for (expected, requirement) in &doors {
            assert_eq!(requirement.source, *expected, "a door wrote another label");
        }
        for &source in IntentSource::ALL {
            let found = doors.iter().filter(|(label, _)| *label == source).count();
            assert_eq!(found, 1, "{source:?} has {found} doors and needs one");
        }
    }

    #[test]
    fn reading_by_label_returns_the_statements_that_arrived_on_it() {
        let intent = ProjectIntent::from_requirements(vec![
            only_requirement(crate::project_intent::explicit_goal("ship it").expect("a goal")),
            inferred("guess-1", "probably a todo app"),
            inferred("guess-2", "probably used for invoices"),
        ]);

        let texts: Vec<&str> = from_source(&intent, IntentSource::Inferred)
            .map(|requirement| requirement.text.as_str())
            .collect();
        assert_eq!(texts, ["probably a todo app", "probably used for invoices"]);
        assert_eq!(from_source(&intent, IntentSource::AgentClaim).count(), 0);
        assert_eq!(
            from_source(&intent, IntentSource::ExplicitUserGoal).count(),
            1
        );
    }
}

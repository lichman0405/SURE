//! Where a project's intent comes from, end to end, and which label travels
//! with it.
//!
//! `crate::intent_model`'s own tests hold the doors one at a time. This file
//! drives them over real directories with a real `sure.yaml` and a real user
//! settings file, because the interesting failures are not in a door: they are
//! in what a whole project's intent adds up to, and in who was allowed to say
//! what.
//!
//! # The claim this file exists to hold
//!
//! P2-T011 acceptance:
//!
//! > Intent sources preserve provenance/trust labels.
//! > Inferred intent cannot satisfy user requirements.
//!
//! The first is a claim about a **label surviving**, and a test can hold it
//! wrongly by reading the label off and agreeing with it. So every test that
//! asserts a label here drives the statement through
//! [`may_claim_full_fulfilment`] as well, which is the gate `P6-T008` will use
//! and the thing the label is actually for.
//!
//! The second is a claim about **what a guess cannot do**, and it has the same
//! shape as the first: `an_inference_..._is_still_not_a_requirement` asserts
//! that adding a guess to an intent leaves the answer to the gate *identical*,
//! rather than asserting a `false` a mutation could produce by breaking the gate
//! for everything.
//!
//! # The one this file exists for
//!
//! `a_project_cannot_grant_itself_the_recording_that_keeps_a_request` is the
//! reason [`observed_user_request`] takes an [`Authority`] rather than a
//! [`Config`]. The same `sure.yaml`, byte for byte, gives opposite answers
//! depending on whether the user's own settings outside the project allowed it —
//! which is `docs/architecture/CONFIG_AUTHORITY.md`'s rule, checked at the one
//! door in this build that would otherwise have acted on a project's word.

// The workspace forbids `unwrap`, `expect` and `panic` in shipped code, because
// a panic is a message nobody chose. A test is the one place they are the point:
// the panic *is* the report, and it names the value that was wrong.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use sure_core::config::{Authority, Layer, ProjectRequest};
use sure_core::discover::{DiscoverOptions, Discovery, discover};
use sure_core::documents::DocumentReport;
use sure_core::intent::{
    IntentSource, ProjectIntent, Requirement, RequirementAuthority, may_claim_full_fulfilment,
};
use sure_core::intent_model::{
    IntentRefusal, agent_claim, documented_goal, from_source, inferred, observed_user_request,
};
use sure_core::project_intent::explicit_goal;
use sure_core::status::{NO_TRUSTED_INTENT_LIMITATION, RequirementClaim};

/// A project directory under the workspace's git-ignored `target/tmp`, and the
/// place the user's own settings would live.
///
/// Unique per call and **never cleared**, which is the pattern this repository
/// settled on after a false report: clearing a fixed path with
/// `let _ = remove_dir_all(..)` and then treating it as fresh fails on Windows,
/// and the test then describes a directory that was never emptied. A path nobody
/// has used before needs no removal. Uniqueness comes from `create_dir`, not
/// from the name, so two processes given the same id cannot collide — a
/// directory that exists is skipped rather than adopted.
///
/// The user settings are a **sibling** of the project directory rather than a
/// file inside it, because that is the whole point of the two layers: a file the
/// project could write is not the user's. It does not exist until a test writes
/// it, which is what makes "the user declared nothing" the ordinary case here as
/// it is in a run.
struct Fixture {
    project: PathBuf,
    user: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = sure_testkit::repository_root()
            .join("target")
            .join("tmp")
            .join("intent sources");
        std::fs::create_dir_all(&base)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", base.display()));
        for _ in 0..1_000 {
            let name = format!("{test}-{}", NEXT.fetch_add(1, Ordering::Relaxed));
            let project = base.join(&name);
            match std::fs::create_dir(&project) {
                Ok(()) => {
                    return Self {
                        user: base.join(format!("{name}.user.yaml")),
                        project,
                    };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", project.display()),
            }
        }
        panic!("no free fixture name under {}", base.display());
    }

    fn write(&self, relative: &str, contents: &str) -> &Self {
        let full = self.project.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", parent.display()));
        }
        std::fs::write(&full, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", full.display()));
        self
    }

    /// Write the user's own settings, outside the project.
    fn user_settings(&self, contents: &str) -> &Self {
        std::fs::write(&self.user, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", self.user.display()));
        self
    }

    /// The two configuration layers as a run reads them.
    ///
    /// Through [`Authority::load`] rather than by building the two files'
    /// contents into values, so that a test cannot pass with a combination the
    /// reader would have refused.
    fn authority(&self) -> Authority {
        Authority::load(&self.project, &self.user)
            .unwrap_or_else(|error| panic!("cannot read this project's settings: {error}"))
    }

    fn discovery(&self) -> Discovery {
        discover(&self.project, &DiscoverOptions::default()).expect("discover")
    }

    fn report(&self) -> DocumentReport {
        DocumentReport::of(&self.discovery())
    }
}

/// Every part of an intent, assembled the way a run would assemble it.
fn intent_of(parts: impl IntoIterator<Item = Vec<Requirement>>) -> ProjectIntent {
    ProjectIntent::from_requirements(parts.into_iter().flatten().collect())
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
fn a_goal_written_into_sure_yaml_is_documentation_and_never_a_user_requirement() {
    // The sentence `docs/adr/0011-project-configuration-is-a-request.md` states
    // and `docs/architecture/CONFIG_REFERENCE.md` repeats about the setting:
    // both are read from a project-controlled file, so both are documentation
    // and neither can become the requirement the project is graded against.
    //
    // The goal is two sentences long on purpose. "Keep the first sentence" is
    // the plausible way to lose the second half of what a project wrote down,
    // and every goal with a full stop in it is a goal that can lose one.
    let fixture = Fixture::new("documented goal");
    fixture
        .write(
            "sure.yaml",
            "project_intent:\n  goal: \"A local bookmark manager. It imports from Firefox.\"\n",
        )
        .write("README.md", "```bash\nnpm test\n```\n");

    let authority = fixture.authority();
    let documented =
        documented_goal(authority.project()).expect("the project documented a goal about itself");
    assert_eq!(
        documented.text, "A local bookmark manager. It imports from Firefox.",
        "the project's own words were edited on the way in"
    );
    assert_eq!(documented.source, IntentSource::ProjectSpec);
    assert_eq!(
        documented.authority(),
        RequirementAuthority::DocumentedInstruction
    );
    assert!(!documented.is_user_requirement());

    // And the label survives into the whole intent, driven through the gate
    // rather than read off: documentation and a README command together still
    // leave SURE unable to say anything about what was asked for.
    let intent = intent_of([vec![documented], fixture.report().as_requirements()]);
    assert_eq!(from_source(&intent, IntentSource::ProjectSpec).count(), 2);
    assert_eq!(intent.user_requirements().count(), 0);
    assert!(!intent.has_user_requirement());
    assert!(intent.is_after_the_fact());
    assert_eq!(intent.requirement_claim(), RequirementClaim::AfterTheFact);
    assert_eq!(intent.caveat(), Some(NO_TRUSTED_INTENT_LIMITATION));
    assert!(
        !may_claim_full_fulfilment(&intent, usize::MAX),
        "documentation became the standard the project is graded against"
    );
}

#[test]
fn every_statement_keeps_the_label_of_the_channel_it_arrived_on() {
    let fixture = Fixture::new("labels");
    fixture
        .write(
            "sure.yaml",
            "project_intent:\n  goal: A local bookmark manager.\n",
        )
        .write("README.md", "```bash\nnpm test\n```\n")
        .user_settings("privacy:\n  full_recording: true\n");
    let authority = fixture.authority();

    let intent = intent_of([
        vec![
            only_requirement(explicit_goal("ship the export feature").expect("a goal")),
            documented_goal(authority.project()).expect("documented"),
            observed_user_request("add CSV export", &authority).expect("granted"),
            agent_claim("claim-1", "login is finished"),
            inferred("guess-1", "probably a todo app"),
        ],
        fixture.report().as_requirements(),
    ]);

    // Six statements, five labels, and every label on the statement that
    // arrived with it. Counted per source and not by position, because a door
    // that wrote the right label in the wrong place would pass an index check.
    let mut labels: Vec<(IntentSource, usize)> = IntentSource::ALL
        .iter()
        .map(|&source| (source, from_source(&intent, source).count()))
        .collect();
    labels.sort_by_key(|(source, _)| source.trust_rank());
    assert_eq!(
        labels,
        vec![
            (IntentSource::ExplicitUserGoal, 1),
            (IntentSource::ObservedUserRequest, 1),
            (IntentSource::ProjectSpec, 2),
            (IntentSource::AgentClaim, 1),
            (IntentSource::Inferred, 1),
        ],
        "a statement arrived with another channel's label"
    );
    assert_eq!(intent.requirements.len(), 6);

    // The two channels that can stand in for the user's goal are the two that
    // got there, and the guess and the claim count towards nothing.
    assert_eq!(intent.user_requirements().count(), 2);
    assert_eq!(intent.agent_claims().count(), 1);
    assert!(!intent.is_after_the_fact());
    assert!(may_claim_full_fulfilment(&intent, 2));
    assert!(!may_claim_full_fulfilment(&intent, 1));
}

#[test]
fn a_project_cannot_grant_itself_the_recording_that_keeps_a_request() {
    // The same bytes on disk both times. What changes is who was allowed to say
    // yes, which is the whole of `docs/architecture/CONFIG_AUTHORITY.md`.
    let fixture = Fixture::new("refused escalation");
    fixture.write(
        "sure.yaml",
        "privacy:\n  full_recording: true\nproject_intent:\n  goal: A local bookmark manager.\n",
    );

    let refused = fixture.authority();
    assert_eq!(
        observed_user_request("add CSV export", &refused),
        Err(IntentRefusal::RecordingNotPermitted)
    );
    let privilege = refused
        .privilege(ProjectRequest::FullRecording)
        .expect("the project asked and left a record");
    assert_eq!(privilege.asked_by, vec![Layer::Project]);
    assert!(!privilege.is_granted());
    assert!(privilege.is_refused_escalation());
    // And the refusal is not silence about anything else: the project's own
    // documented goal still reads exactly as it did.
    assert!(documented_goal(refused.project()).is_some());

    // The control. Nothing about the project changed; the user allowed it.
    fixture.user_settings("privacy:\n  full_recording: true\n");
    let granted = fixture.authority();
    let request =
        observed_user_request("add CSV export", &granted).expect("the user allowed recording");
    assert_eq!(request.source, IntentSource::ObservedUserRequest);
    assert_eq!(
        request.authority(),
        RequirementAuthority::UserRequirement,
        "a kept request is worth exactly what a goal that was typed is worth"
    );

    let intent = intent_of([vec![request]]);
    assert!(!intent.is_after_the_fact());
    assert!(may_claim_full_fulfilment(&intent, 1));
    assert!(!may_claim_full_fulfilment(&intent, 0));
}

#[test]
fn an_inference_with_the_users_own_words_in_it_is_still_not_a_requirement() {
    // The T17 failure in miniature: SURE guessing exactly right, in the user's
    // own words. It is the sharpest version of the case, because every reason
    // to promote it is present except the one that decides — the channel.
    let words = "stop the nightly job from double-charging";
    let stated = only_requirement(explicit_goal(words).expect("a goal"));
    let guessed = inferred("guess-1", words);
    assert_eq!(stated.text, guessed.text, "the two say the same thing");

    let without = intent_of([vec![stated.clone()]]);
    let with = intent_of([vec![stated, guessed]]);

    // Nothing about what SURE may claim changes when the guess is added. Not
    // "the guess is refused" — that would pass even if the gate were broken for
    // everything — but "the answer is the one the user's own words got".
    for evidence in 0..=3 {
        assert_eq!(
            may_claim_full_fulfilment(&with, evidence),
            may_claim_full_fulfilment(&without, evidence),
            "a guess counted as a requirement at {evidence} pieces of fresh evidence"
        );
    }
    assert!(!may_claim_full_fulfilment(&with, 0));
    assert!(may_claim_full_fulfilment(&with, 1));
    assert_eq!(with.user_requirements().count(), 1, "the guess was counted");
    assert_eq!(from_source(&with, IntentSource::Inferred).count(), 1);
}

#[test]
fn an_intent_of_nothing_but_guesses_can_never_claim_fulfilment() {
    let intent = intent_of([vec![
        inferred("guess-1", "probably a todo app"),
        inferred("guess-2", "probably used for invoices"),
        agent_claim("claim-1", "login is finished"),
    ]]);

    for evidence in 0..=5 {
        assert!(
            !may_claim_full_fulfilment(&intent, evidence),
            "{evidence} pieces of fresh evidence were enough for an intent with no user requirement"
        );
    }
    assert!(intent.is_after_the_fact());
    assert_eq!(intent.caveat(), Some(NO_TRUSTED_INTENT_LIMITATION));
}

#[test]
fn three_statements_that_say_the_same_thing_stay_three_statements() {
    // The way provenance is lost in practice is not a wrong label. It is a
    // deduplication: two sentences that match are treated as one, and whichever
    // label survives is the higher one. That promotion is the thing the model
    // has no function to perform, and this is the test that would fail if one
    // were ever added.
    let fixture = Fixture::new("no merging");
    fixture.write("sure.yaml", "project_intent:\n  goal: add CSV export\n");
    let authority = fixture.authority();

    let intent = intent_of([vec![
        documented_goal(authority.project()).expect("documented"),
        inferred("guess-1", "add CSV export"),
        agent_claim("claim-1", "add CSV export"),
    ]]);

    assert_eq!(intent.requirements.len(), 3, "two statements were merged");
    assert_eq!(from_source(&intent, IntentSource::ProjectSpec).count(), 1);
    assert_eq!(from_source(&intent, IntentSource::Inferred).count(), 1);
    assert_eq!(from_source(&intent, IntentSource::AgentClaim).count(), 1);
    assert_eq!(
        intent.user_requirements().count(),
        0,
        "a statement written by the project was promoted"
    );
    assert!(!may_claim_full_fulfilment(&intent, 3));
    assert!(intent.is_after_the_fact());
}

#[test]
fn every_refusal_says_that_nothing_was_kept() {
    // The rule `crate::store::error` states for its own variants: a user who
    // reads a failure has to be able to tell what did not happen. For a door
    // that keeps nothing, that means the sentence has to say so.
    for error in [IntentRefusal::RecordingNotPermitted, IntentRefusal::NoWords] {
        let text = error.to_string();
        assert!(
            text.contains("so nothing was kept"),
            "{error:?} does not say what SURE did instead: {text}"
        );
    }
    // And the one a user can act on says where to act. A refusal that named no
    // remedy would leave them with a project file that asks for recording and no
    // way to find out why nothing happened.
    assert!(
        IntentRefusal::RecordingNotPermitted
            .to_string()
            .contains("your own SURE settings")
    );
}

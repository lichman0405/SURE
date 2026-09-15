//! The checks a discovered project's own declarations turn into.
//!
//! This is step 4 of `docs/architecture/CHECK_PIPELINE.md` — *"Plan
//! static/dynamic checks and execution trust requirements."* — and it is the
//! first place in the product where a check is proposed.
//! [`crate::schedule`] is the machinery that orders and gates proposals, and
//! until this module existed nothing handed it one; `tests/check_schedule.rs`
//! says in its own documentation that it is written to fail on the day that
//! changed, and that day is `P4-T002`.
//!
//! # It adds no file access of its own
//!
//! Every fact a check here rests on is a field of a discovery result. Nothing in
//! this module opens a manifest, resolves a dependency, looks at
//! `node_modules`, or runs anything: the reading is [`crate::discover`]'s job
//! and this layer only decides what the reading means for a plan. That is not a
//! style preference — a proposer that read a file the scan had not walked would
//! be able to propose a check on evidence the report cannot point at, and
//! `docs/architecture/EVIDENCE_MODEL.md` calls an anchorless claim unsupported.
//!
//! # The two sentences this layer is arranged around
//!
//! **A check SURE would run is gated by the execution mode, and the gating
//! happens elsewhere.** Every check proposed here declares an
//! [`ActionKind`](sure_domain::execution::ActionKind) that
//! [`executes_project_code`](sure_domain::execution::ActionKind::executes_project_code)
//! answers `true` for, so under
//! [`InspectOnly`](sure_domain::execution::ExecutionMode::InspectOnly) every one
//! of them is refused by [`decide`](sure_domain::execution::decide) and comes
//! back from [`CheckSchedule`](crate::schedule::CheckSchedule) as an entry that
//! would not run. Nothing in this module makes that decision — it cannot, since
//! a decision is not a property of a check — and
//! `a_declared_check_runs_only_under_a_mode_that_allows_it` is where that is
//! held rather than asserted.
//!
//! **A command the project does not have never becomes a pass.** This is the
//! half that has no plan entry to hide in. Where a project gives SURE no command
//! for a check, there is nothing to schedule, and the honest answer is a
//! [`MissingCommand`]: an identity, the title the check would have had, and why
//! there is no command for it. [`MissingCommand::not_checked`] turns one into a
//! [`CheckResult`] by way of the domain's own `not_run` constructor, so the only
//! status it can produce is a *skipped* one — and the test
//! `every_missing_command_is_skipped_and_none_of_them_is_a_pass` says so over
//! the whole vocabulary of reasons rather than over the ones somebody thought
//! of.
//!
//! # The vocabulary gap this layer exists to name
//!
//! [`NotCheckedReason`] is frozen and has ten variants, and **none of them means
//! "the project declares this and what it declares is not something SURE can
//! run"**. The nearest is [`NotApplicable`](NotCheckedReason::NotApplicable) —
//! *"This check does not apply to your project."* — which is close for a check
//! the project declares no command for and false for one where the project
//! declared a command and wrote it in a shape that is not a command.
//!
//! [`MissingKind`] is SURE's own answer, and it is the shape
//! [`crate::browser`]'s `AbsenceReason` already has: **the frozen reason is what
//! a report groups by, and SURE's own sentence is what it prints.** The two are
//! separate because they answer separate questions, and because the frozen
//! enum's `is_scope_limit` is load-bearing —
//! [`blocks_green`](sure_domain::status::CheckResult::blocks_green) reads it to
//! decide whether a critical check that did not run holds the run out of green —
//! so the choice is not free and each variant below argues for its own.
//!
//! # What it does not do
//!
//! **It does not decide whether a check passes.** There is no result here except
//! the skipped one a missing command produces, and no path from this module to
//! [`CheckStatus`](sure_domain::status::CheckStatus) other than through
//! `CheckResult::not_run`.
//!
//! **It does not run anything.** Nothing here builds a
//! [`Command`](std::process::Command); the execution trust requirements the
//! acceptance names are the [`ActionKind`](sure_domain::execution::ActionKind)
//! each proposal declares, which is the same thing
//! `tests/spawn_sites.rs` counts the callers of.
//!
//! **It is not the only proposer there will be.** `P4-T003` (Python) and
//! `P4-T004` (Rust) add siblings to [`node`], and the reason the identifier
//! scheme lives here rather than in any one of them is that the identifiers
//! share a namespace: two ecosystems describing one directory must not name one
//! check twice.
//!
//! **It witnesses nothing about whether these checks are the right ones.** A
//! `package.json` that declares `"test": "true"` gets a test check that passes,
//! and nothing here can tell. Choosing good checks is not a thing a table of
//! conventions can do, and the module says so rather than implying otherwise —
//! the same limit [`crate::browser`] states about its drivers.

pub mod node;

use sure_domain::ids::{CheckId, FingerprintId};
use sure_domain::severity::Severity;
use sure_domain::status::{CheckResult, NotCheckedReason};

use crate::fingerprint::digest::Digest;

/// The domain tag for a check identifier.
///
/// A version is in the name because the recipe below is what makes an old
/// result's identifier findable in a new run, and a recipe that changed without
/// saying so would silently orphan every stored result. Changing this string is
/// the honest way to change the recipe, and it is a decision rather than an
/// edit.
const ID_DOMAIN: &str = "sure.check-id.v1";

/// How many hexadecimal characters of the digest an identifier carries.
///
/// Sixty-four bits, which is not a security boundary and is not treated as one.
/// What actually protects against two components sharing an identifier is not
/// this number but the fact that a collision is *reported*: two checks with one
/// identifier end up in [`CheckSchedule::duplicates`](crate::schedule::CheckSchedule::duplicates)
/// rather than being silently merged, so the failure mode of a truncated digest
/// is a visible one. Finding a collision takes on the order of 2^64 hashes for
/// a component path somebody else chose, which is the reason the truncation is
/// affordable at all.
const ID_HEX: usize = 16;

/// The identifier a check for a component and a tag always has.
///
/// **Deterministic on purpose, and that is the whole of its job.** A check's
/// identity is what joins a result from one run to the plan that proposed it in
/// another — `docs/architecture/CHECK_PIPELINE.md` steps 11 and 12 are the
/// repair contract and the re-check, and neither can look a check up if the
/// second run named it something else. `CheckId::generate` is therefore not
/// usable here and its absence from this file is deliberate.
///
/// `component` is the file the check is about — a manifest path, as the scan
/// spells it — and `tag` is the readable half: `nodetest`, and one day
/// `pythonlint`. **The tag is not trusted**: everything outside `[a-z0-9]` is
/// dropped, and the digest is taken over the raw text, so two tags that read
/// alike after dropping (`node-test` and `nodetest`) still get different
/// identifiers.
///
/// # Panics
///
/// Does not, and the reason it can be written as though it cannot is checked
/// rather than asserted:
/// `an_identifier_is_well_formed_for_every_tag_and_component_this_can_be_given`
/// sweeps components and tags with spaces, punctuation, Unicode, path
/// separators, the empty string and lengths past the Windows path limit.
#[allow(
    clippy::expect_used,
    reason = "the body is the tag with every character outside [a-z0-9] dropped, \
              then ID_HEX lowercase hexadecimal characters, under the literal \
              prefix `chk_` — a string CheckId::parse cannot reject. The sweep in \
              `an_identifier_is_well_formed_for_every_tag_and_component_this_can_be_given` \
              holds that over the hostile inputs rather than this sentence \
              asserting it, and a panic here would be a bug this crate's \
              `unwrap_used` rule is right to forbid"
)]
#[must_use]
pub fn check_id(component: &str, tag: &str) -> CheckId {
    let mut digest = Digest::new(ID_DOMAIN);
    digest.field(component).field(tag);
    let digest = digest.finish();

    let mut body: String = tag
        .chars()
        .filter(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        .collect();
    body.extend(digest.chars().take(ID_HEX));

    CheckId::parse(format!("{}_{body}", CheckId::KIND.prefix()))
        .expect("the body is built from [a-z0-9] and lowercase hexadecimal")
}

/// Why SURE has no command for a check it would otherwise propose.
///
/// See the module documentation for why this type exists at all rather than a
/// [`NotCheckedReason`] alone: the frozen vocabulary is what a report groups by,
/// this is what it prints, and the two answer different questions.
///
/// **The variants are the three ways a project can leave SURE without a
/// command**, and they are not interchangeable. A project that declares nothing
/// is a project whose shape puts the check out of scope; a project that declares
/// something unusable, or declares it and cannot say what runs it, has a defect
/// in it — the difference decides whether a critical check holds the run out of
/// green, and that is what each variant's reason is argued for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingKind {
    /// Nothing in the project declares a way to do this.
    ///
    /// [`NotApplicable`](NotCheckedReason::NotApplicable), and it is the one
    /// variant where the frozen enum's own sentence is true: a check SURE knows
    /// how to perform against a project of this shape does not apply to a
    /// project that declares no command for it. It is a scope limit, so a
    /// critical check that hits it does **not** hold the run out of green —
    /// which is the right answer, because a project without a `lint` script is
    /// not a project SURE should refuse to call green.
    NotDeclared,
    /// The project declares one, and what it declares is not a command.
    ///
    /// A `"test": ["jest"]` in `package.json`. npm refuses to run it, SURE
    /// refuses to guess at it, and the name is recorded rather than dropped —
    /// [`Package::scripts_not_commands`](crate::discover::node::Package::scripts_not_commands)
    /// says in its own documentation that a script which is there and unusable
    /// is not a script that is not there, and this variant is where that
    /// distinction survives into the report.
    ///
    /// **Not a scope limit.** The vocabulary has no word for this and the
    /// nearest one is used; see [`Self::not_checked_reason`] for why that word
    /// is [`UnknownReason`](NotCheckedReason::UnknownReason) rather than
    /// [`NotApplicable`](NotCheckedReason::NotApplicable).
    NotACommand,
    /// The project declares one and does not say what runs it.
    ///
    /// `why` is a constant sentence from the discovery — the one that says
    /// *which* of the two ways the project failed to name a runner, because
    /// [`Managers::agreed`](crate::discover::node::Managers::agreed) answers
    /// `None` for two different reasons and says in its own documentation that a
    /// caller which renders the two the same way throws away
    /// [`disagreement`](crate::discover::node::Managers::disagreement).
    NoRunner {
        /// The discovery's own sentence, never project text.
        why: &'static str,
    },
}

/// The sentence for the case where nothing at all names a runner.
///
/// A constant rather than a built string, for the reason every sentence in this
/// repository is: a project must not be able to write a line SURE says.
const NOTHING_NAMES_A_RUNNER: &str =
    "Nothing in this project says which package manager runs its scripts, and SURE does not guess.";

impl MissingKind {
    /// Every kind, so a rule can be checked over the whole set.
    ///
    /// The same technique as
    /// [`AbsenceReason::ALL`](crate::browser::AbsenceReason::ALL), and for the
    /// same reason: a fourth kind added later that landed on the wrong side of
    /// the scope-limit split fails a test rather than being noticed in review.
    pub const ALL: &'static [Self] = &[
        Self::NotDeclared,
        Self::NotACommand,
        Self::NoRunner {
            why: NOTHING_NAMES_A_RUNNER,
        },
    ];

    /// The frozen reason this maps onto.
    ///
    /// **`NotACommand` and `NoRunner` both answer
    /// [`UnknownReason`](NotCheckedReason::UnknownReason), and that word is
    /// wrong.** SURE knows exactly why neither ran. The vocabulary has no word
    /// for *the project declared this and what it declared is not something
    /// SURE can run*, and the three candidates that come close are each false in
    /// a way that costs something:
    ///
    /// - [`NotApplicable`](NotCheckedReason::NotApplicable) says the check does
    ///   not apply to your project, which is a lie a person would act on — they
    ///   would not go and look at their manifest.
    /// - [`ToolUnavailable`](NotCheckedReason::ToolUnavailable) says the tool is
    ///   not installed on this computer, which is a claim about the machine and
    ///   is false whenever `npm` is sitting right there.
    /// - [`UnsupportedStack`](NotCheckedReason::UnsupportedStack) says the
    ///   project is not on a stack SURE can check, which is false: this is the
    ///   stack it checks best.
    ///
    /// What is left is a reason that is false about SURE's own state instead of
    /// false about the project or the machine — and it is the only one of the
    /// four that does not class a defect the user can fix as a scope limit. A
    /// `"test": ["jest"]` is a broken manifest, and a critical test check that
    /// cannot run because of one should keep the run out of green. The sentence
    /// a person reads is [`Self::plain_explanation`]'s and is true; this is the
    /// word a report groups by, and it is recorded in `progress/DECISIONS.md`.
    #[must_use]
    pub const fn not_checked_reason(&self) -> NotCheckedReason {
        match self {
            Self::NotDeclared => NotCheckedReason::NotApplicable,
            Self::NotACommand | Self::NoRunner { .. } => NotCheckedReason::UnknownReason,
        }
    }

    /// SURE's own sentence, which is the one a report prints.
    #[must_use]
    pub fn plain_explanation(&self) -> String {
        match self {
            Self::NotDeclared => "Your project does not declare a way to do this.".to_owned(),
            Self::NotACommand => {
                "What your project declares for this is not a command SURE can run.".to_owned()
            }
            Self::NoRunner { why } => (*why).to_owned(),
        }
    }
}

/// A check SURE would have proposed and could not, because the project gives it
/// no command to run.
///
/// **The acceptance sentence this type exists for is "Missing commands are not
/// passes."** A check with no command produces no plan entry, so the failure
/// this guards against is not a wrong verdict but a *silence*: a report built
/// from the checks that exist would say nothing at all about tests in a project
/// that has none, and a reader would take the absence of a row for the absence
/// of a problem.
///
/// So a missing command is a value with the identity the check would have had,
/// and [`Self::not_checked`] turns it into the only kind of
/// [`CheckResult`] this crate can produce for a check that did not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingCommand {
    id: CheckId,
    title: String,
    component: String,
    severity: Severity,
    critical: bool,
    kind: MissingKind,
}

impl MissingCommand {
    /// A missing command, for a check that would have been about `component`.
    #[must_use]
    pub fn new(
        id: CheckId,
        title: impl Into<String>,
        component: impl Into<String>,
        severity: Severity,
        critical: bool,
        kind: MissingKind,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            component: component.into(),
            severity,
            critical,
            kind,
        }
    }

    /// The identity the check would have had.
    ///
    /// The same identity the proposal would have carried, because it is derived
    /// from the component and the role and not from whether a command exists.
    /// Anything else would make the answer to "was this check run?" depend on
    /// the project having been changed between the two runs being compared.
    #[must_use]
    pub const fn id(&self) -> &CheckId {
        &self.id
    }

    /// The title the check would have had.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The file SURE read to find out.
    #[must_use]
    pub fn component(&self) -> &str {
        &self.component
    }

    /// How bad it would have been if this check had not been satisfied.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Whether the check would have held the run out of green.
    #[must_use]
    pub const fn critical(&self) -> bool {
        self.critical
    }

    /// Why there is no command.
    #[must_use]
    pub const fn kind(&self) -> &MissingKind {
        &self.kind
    }

    /// This check as the result of not running it.
    ///
    /// **`CheckResult::not_run` is the only constructor this reaches**, which is
    /// what makes the acceptance sentence structural rather than a convention:
    /// the status is `Skipped`, the evidence class is `Unknown` because the
    /// domain's constructor says a check that did not run established nothing,
    /// and there is no way to spell a pass here even by mistake.
    ///
    /// **The printed sentence is replaced.** `not_run` fills `reason` with
    /// `not_checked_reason.plain_explanation()`, which is the frozen
    /// vocabulary's generic sentence for the word it was given; for
    /// [`MissingKind::NotDeclared`] that sentence is true and for the other two
    /// it is not, so the line a person reads is SURE's own and the vocabulary's
    /// word stays in `not_checked_reason` for anything that groups by it.
    /// `browser.rs` does the same thing for the same reason.
    #[must_use]
    pub fn not_checked(&self, project_fingerprint: &FingerprintId) -> CheckResult {
        let mut result = CheckResult::not_run(
            self.id.clone(),
            self.title.clone(),
            self.severity,
            self.critical,
            self.kind.not_checked_reason(),
            project_fingerprint.clone(),
        );
        result.reason = self.kind.plain_explanation();
        result
    }

    /// One line for a report, in the shape a scheduled check's is.
    #[must_use]
    pub fn plain_description(&self) -> String {
        format!(
            "{} - not checked. {}",
            self.title,
            self.kind.plain_explanation()
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::evidence::EvidenceClass;
    use sure_domain::status::CheckStatus;

    /// Component paths and tags chosen to be hostile rather than representative.
    ///
    /// Every one of these is a string a scan can hand over: a member directory
    /// with a space in it, one with Unicode, one that is a Windows path with
    /// backslashes, one with `..` in it, one with a `.` or a `-` where the tag
    /// would have had a letter, an empty one, and one longer than the Windows
    /// path limit. The tags are the ones this layer builds plus the shapes that
    /// would break a `<prefix><tag><hex>` recipe if the tag were trusted.
    const HOSTILE: &[&str] = &[
        "",
        ".",
        "..",
        "package.json",
        "apps/web/package.json",
        "apps/my app/package.json",
        "apps/包/package.json",
        "apps\\win\\package.json",
        "apps/web/../../etc/package.json",
        "NODE_TEST",
        "node-test",
        "nodetest",
        "node test",
        "node.test",
        "chk_nodetest",
        "chk_",
        "_",
        "0",
        "9999999999",
    ];

    fn long_path() -> String {
        let mut path = String::from("apps");
        for index in 0..40 {
            path.push_str(&format!("/member-{index}"));
        }
        path.push_str("/package.json");
        path
    }

    #[test]
    fn an_identifier_is_well_formed_for_every_tag_and_component_this_can_be_given() {
        // The claim `check_id`'s `expect` rests on, held over the inputs rather
        // than asserted in a sentence. A panic here is the failure this test
        // exists to catch before it can reach a project.
        let long = long_path();
        let components = HOSTILE
            .iter()
            .copied()
            .chain(std::iter::once(long.as_str()));

        let mut built = 0;
        for component in components {
            for tag in HOSTILE {
                let id = check_id(component, tag);
                // Parsed back, so this is a claim about the identifier's *text*
                // and not about the constructor having accepted it once.
                let parsed = CheckId::parse(id.as_str().to_owned())
                    .unwrap_or_else(|error| panic!("{id} is not a CheckId: {error}"));
                assert_eq!(parsed, id);
                assert!(id.as_str().starts_with("chk_"), "{id}");
                assert!(
                    id.as_str().len() > "chk_".len(),
                    "the body is empty, which is the one shape a CheckId refuses: {id}"
                );
                built += 1;
            }
        }
        assert_eq!(
            built,
            (HOSTILE.len() + 1) * HOSTILE.len(),
            "the sweep did not visit every pair, so it is not the claim it says it is"
        );
    }

    #[test]
    fn an_identifier_is_the_same_every_time_and_different_for_every_check() {
        // Two claims, and the second is the one with teeth: a tag that reads the
        // same after everything outside `[a-z0-9]` is dropped must still get its
        // own identifier, because the digest is taken over the raw text. Without
        // that, `node-test` and `nodetest` would be one check.
        let ids: Vec<CheckId> = HOSTILE
            .iter()
            .map(|tag| check_id("apps/web/package.json", tag))
            .collect();
        for (index, id) in ids.iter().enumerate() {
            assert_eq!(
                *id,
                check_id("apps/web/package.json", HOSTILE[index]),
                "the same component and tag gave two identifiers"
            );
        }

        let mut sorted = ids.clone();
        sorted.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            ids.len(),
            "two tags share an identifier, so one check would be dropped as a \
             duplicate of the other: {ids:?}"
        );

        // And the component is in the digest as well as the tag, which is the
        // half a tag-only recipe would lose: the same role in two members is two
        // checks.
        assert_ne!(
            check_id("apps/web/package.json", "nodetest"),
            check_id("apps/api/package.json", "nodetest")
        );
    }

    #[test]
    fn a_component_that_is_not_there_is_not_a_component_with_no_command() {
        // The false green this type is arranged against, stated as a value: a
        // missing command is a *reason*, and a caller cannot build one without
        // saying which reason. An empty component is refused by the discovery
        // rather than here — what is held here is that the three kinds are three
        // values, so a caller that has one cannot be read as having another.
        assert_eq!(MissingKind::ALL.len(), 3);
        assert_ne!(MissingKind::NotDeclared, MissingKind::NotACommand);
        assert_ne!(
            MissingKind::NotDeclared,
            MissingKind::NoRunner {
                why: NOTHING_NAMES_A_RUNNER
            }
        );
    }

    #[test]
    fn every_missing_command_is_skipped_and_none_of_them_is_a_pass() {
        // The acceptance sentence, over the whole vocabulary rather than over
        // the kinds somebody thought of. A fourth kind added later that landed
        // on anything but `Skipped` fails here.
        let fingerprint = FingerprintId::generate();
        for kind in MissingKind::ALL {
            let missing = MissingCommand::new(
                check_id("package.json", "nodetest"),
                "run the tests",
                "package.json",
                Severity::MustFix,
                true,
                kind.clone(),
            );
            let result = missing.not_checked(&fingerprint);

            assert_eq!(result.status, CheckStatus::Skipped, "{kind:?}");
            assert!(!result.status.is_green(), "{kind:?} was green");
            assert!(
                !result.status.produced_a_result(),
                "{kind:?} claimed to have checked the project"
            );
            assert_eq!(result.evidence_class, EvidenceClass::Unknown);
            assert_eq!(result.not_checked_reason, Some(kind.not_checked_reason()));
            assert_eq!(result.project_fingerprint, fingerprint);

            // The sentence a person reads is SURE's, and it is never empty. This
            // is the half the frozen vocabulary cannot carry: for two of the
            // three kinds the generic sentence is false, so the two must differ.
            assert_eq!(result.reason, kind.plain_explanation());
            assert!(!result.reason.trim().is_empty(), "{kind:?} says nothing");

            // And the scope-limit split, which is the part that decides whether
            // a critical check holds the run out of green.
            assert_eq!(
                result.blocks_green(),
                !kind.not_checked_reason().is_scope_limit(),
                "{kind:?} blocks green if and only if its domain reason is a gap"
            );

            // And the one-line description, which is the other place the
            // sentence has to survive. **The first run of `P4-T002`'s mutation
            // set dropped the explanation from this line and every test
            // passed**, because nothing read it: a report showing
            // `run the tests - not checked.` and stopping is the exact silence
            // this type exists against, printed one layer further out than the
            // type can see.
            let line = missing.plain_description();
            assert!(line.contains(missing.title()), "{line}");
            assert!(line.contains(&kind.plain_explanation()), "{line}");
            assert_ne!(line, missing.title(), "{line}");
        }

        // The specific split, written out so that moving one kind across it is a
        // failing test rather than a plausible-looking diff: a project that
        // declares nothing is out of scope, and a project that declares
        // something SURE cannot run is a defect in the project.
        let scope_limits: Vec<&MissingKind> = MissingKind::ALL
            .iter()
            .filter(|kind| kind.not_checked_reason().is_scope_limit())
            .collect();
        assert_eq!(scope_limits, vec![&MissingKind::NotDeclared]);
    }

    #[test]
    fn the_sentence_a_person_reads_is_not_the_vocabularys_sentence_where_that_one_is_false() {
        // Named directly because it is the reason `not_checked` overwrites the
        // field at all, and a reader who found the assignment odd deserves the
        // failing test that says what it is for. Both of these answer
        // `UnknownReason`, whose sentence is that SURE does not know why a check
        // did not run — which would be a false statement about SURE's own state
        // in place of a true one about the project's.
        for kind in [
            MissingKind::NotACommand,
            MissingKind::NoRunner {
                why: NOTHING_NAMES_A_RUNNER,
            },
        ] {
            assert_eq!(kind.not_checked_reason(), NotCheckedReason::UnknownReason);
            assert_ne!(
                kind.plain_explanation(),
                kind.not_checked_reason().plain_explanation(),
                "{kind:?} would print the generic sentence, which is false for it"
            );
        }

        // And each of the three says something the other two do not, so a report
        // showing one where another belongs is a visible difference rather than
        // three ways of saying "not checked".
        let sentences: Vec<String> = MissingKind::ALL
            .iter()
            .map(MissingKind::plain_explanation)
            .collect();
        assert_eq!(sentences.len(), 3);
        for (index, sentence) in sentences.iter().enumerate() {
            assert!(
                !sentences[index + 1..].contains(sentence),
                "two missing kinds render identically: {sentence}"
            );
        }
    }
}

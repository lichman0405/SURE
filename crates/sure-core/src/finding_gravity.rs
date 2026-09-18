//! The one rule that decides how serious a detector finding is.
//!
//! `P7-T011`'s acceptance: *the rule that decides between `note`, `can_fix_later`
//! and `must_fix` for a detector finding is written where the detectors and the
//! aggregator can both be read against it.* This is that rule, and this module
//! is its only home.
//!
//! # Why it is a module and not a constant
//!
//! Until this task every false-completion detector hard-coded `Severity::Note`
//! in its own category table — seventeen literal occurrences in the shipping
//! code of six files, and not one shipping line in any of them naming another
//! level — and [`crate::false_completion_aggregator::aggregate`] filed every one
//! of the proposals they built as style noise. Measured on 2026-09-18 by running
//! them over the eleven fixture apps this repository ships: all twenty-five
//! proposals the five scanners produced were `Note`, `critical = false`,
//! `Inference`, and all twenty-five landed in style noise — while
//! `evaluation/acceptance-manifest.json` requires `must_fix` for six
//! release-blocking cases whose fixture app exists, none of which could reach it.
//! The fixtures were right and the calibration was missing.
//!
//! A rule copied into six tables is six rules that agree by coincidence, so the
//! rule is written once and takes what a detector actually knows.
//!
//! # What the rule is asked
//!
//! [`gravity_of`] takes four things and nothing else:
//!
//! 1. the [`CheckReason`] the proposal will carry — the rule reads
//!    [`CheckReason::names_something`] and [`CheckReason::anchor`] out of it
//!    rather than being handed a boolean, so a detector cannot claim an anchor
//!    it does not have;
//! 2. the [`EvidenceClass`] the check's result would carry, which the rule
//!    **reads and never raises** — see *What is not claimed*;
//! 3. [`Reach`], whether what was found can reach a user of the project;
//! 4. [`GapKind`], the detector's own reading of what kind of gap it found.
//!
//! # The rule
//!
//! Four gates, then the gap:
//!
//! | condition | severity | rationale |
//! |---|---|---|
//! | the reason names nothing, or its anchor is missing or uncheckable | `note` | informational |
//! | the evidence class is `unknown` | `note` | informational |
//! | the finding cannot reach a user ([`Reach::NotProduction`]) | `note` | informational |
//! | the finding is about declared intent ([`GapKind::DeclaredIntent`]) | `note` | informational |
//! | [`GapKind::UnfinishedMarker`], reachable | `can_fix_later` | non-blocking improvement |
//! | [`GapKind::UnrealContent`], reachable | `should_fix_first` | reliability or quality risk |
//! | [`GapKind::SubstitutedAction`], reachable | `must_fix` | blocks hand-off |
//!
//! The first three gates are the same idea three times: a finding nobody can act
//! on is not a finding. If it names nothing, or points at a place a reader cannot
//! open, or rests on a check whose result would be worth nothing, or describes
//! something that cannot reach a user, then the honest answer is `note` — not
//! because the underlying worry is small, but because SURE has not earned the
//! right to spend a user's attention on it.
//!
//! **`UnfinishedMarker` is `can_fix_later` and not `must_fix` on purpose, and the
//! corpus is why.** `fixtures/adversarial/fake-auth/scenario.json` requires
//! `must_fix` for `CandidateCategory::Todo` anchored at `src/auth.js`, and
//! `fixtures/adversarial/demo-analytics/scenario.json` requires `should_fix_first`
//! for the same category anchored at `src/metrics.js`. Both are `TODO` in
//! production code in the same context with the same evidence class, so **no
//! function of a proposal's own fields can satisfy both**, and a rule that made
//! every production TODO a `must_fix` would fire on every repository ever
//! written. [`crate::candidate_scanner`] sees the word and where it stands; it
//! does not read what the TODO is on. `can_fix_later` is what it has earned, and
//! the release-blocking fixtures reach `must_fix` through the detector that saw
//! the substituted action itself.
//!
//! # What is not claimed
//!
//! **The rule does not decide whether a finding is a defect**, and a `must_fix`
//! here does not mean "this is broken". It answers the same question
//! [`CheckProposal::severity`](crate::schedule::CheckProposal::severity) asks —
//! *how bad is it if this check is not satisfied* — and in this pipeline a
//! candidate is what `describe_candidates` calls it: *something to look at, not a
//! defect SURE has established*.
//!
//! **The rule never touches the evidence class.** A `must_fix` finding here is
//! still `EvidenceClass::Inference` when the detector only pattern-matched, and
//! `fixtures/adversarial/fake-payment/scenario.json` names `inference` as the
//! required class beside `must_fix` as the required severity. Severity is raised
//! by anchoring a claim to a place in the project, never by relabelling a guess
//! as an observation. `EvidenceClass::can_alone_support_must_fix` still says what
//! it says; this rule is a rule about a *candidate*, and a candidate has not been
//! asserted as a [`Finding`](sure_domain::finding::Finding) at all.
//!
//! **The rule does not set `critical`.** Whether a project cannot be handed off
//! when a check fails is a separate question with its own field, and nothing here
//! changes it. That matters at the verdict: `CheckResult::blocks_green` reads
//! `critical`, so raising a candidate's severity does not by itself turn a
//! verdict red.
//!
//! # Who reads it
//!
//! [`crate::candidate_scanner`], [`crate::noop_heuristics`],
//! [`crate::demo_data_heuristics`], [`crate::route_consistency`],
//! [`crate::ui_action_bridge`] and [`crate::intent_implementation`] ask it for
//! every proposal they build, and
//! [`crate::false_completion_aggregator::aggregate`] reads its answer through
//! [`is_informational`] rather than deciding for itself which candidates are
//! style noise. `tests/finding_severity_rule.rs` is the test that fails when a
//! detector goes back to writing a severity literal of its own.

use sure_domain::evidence::EvidenceClass;
use sure_domain::finding::SeverityRationale;
use sure_domain::severity::Severity;

use crate::candidate_context::CandidateContext;
use crate::schedule::CheckReason;

/// What kind of gap a detector found.
///
/// **This is the detector's own reading of what it saw, and it is the only
/// judgement the rule asks a detector to make.** A detector that has to name one
/// of these has to have looked: the three variants differ in what the project
/// would have to do to close the gap, which is what severity is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GapKind {
    /// Work the author marked as unfinished: a `TODO`, a `placeholder`, a stub.
    ///
    /// The code says out loud that it is not finished. Nothing here claims the
    /// work was faked; it claims the work is not done, and that is a
    /// non-blocking improvement until something connects it to a user path.
    UnfinishedMarker,
    /// Content that is not real, presented by code that is: demo datasets,
    /// sample records, and identifiers standing in for a measurement.
    ///
    /// The code runs and produces a result a user reads. What is wrong is that
    /// the numbers are constants — a reliability risk, not yet a claim that
    /// anything was faked to a customer.
    UnrealContent,
    /// Code standing in for an action the product tells a user it performs: a
    /// sandbox key where a provider call belongs, a constant success where an
    /// authorisation belongs, a declared button whose behaviour SURE has no
    /// evidence of, a path the frontend calls that no backend declares.
    ///
    /// **This is the level the release-blocking fixtures are about**, and the
    /// reason it is `must_fix` rather than `should_fix_first` is that the gap is
    /// not a matter of quality: a user or a caller is being told something
    /// happened, and SURE has not established that it did. Do not hand the
    /// project off while that is true.
    SubstitutedAction,
    /// A statement about what the project was asked to do rather than about code
    /// SURE read: a documented instruction, an agent's claim, or a requirement
    /// with no implementation evidence.
    ///
    /// Its anchor names the requirement's own identifier, which is not a file in
    /// the project, and the claim it carries — *SURE could not match this to
    /// code* — is about SURE's reading rather than about the project. It stays a
    /// note whatever else is known about it; the corpus requires exactly that
    /// (`evaluation/acceptance-manifest.json`: `missing-user-intent` is `note`).
    DeclaredIntent,
}

/// Whether what a detector found can reach a user of the project.
///
/// The two halves of [`CandidateContext`], named for the question this rule
/// asks rather than for the directory the file happened to sit in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reach {
    /// Production or product code: a path a user or a caller of the project
    /// walks through.
    Production,
    /// Tests, examples, documentation or mock fixtures — and anything else that
    /// cannot reach a user, including a claim about declared intent.
    NotProduction,
}

impl From<CandidateContext> for Reach {
    /// A candidate in product code can reach a user; one in a test, an example,
    /// a document or a mock fixture cannot.
    fn from(context: CandidateContext) -> Self {
        match context {
            CandidateContext::Product => Self::Production,
            CandidateContext::Test
            | CandidateContext::Example
            | CandidateContext::Doc
            | CandidateContext::MockFixture => Self::NotProduction,
        }
    }
}

/// The severity a finding carries and the rationale that has to travel with it.
///
/// The two are built together and cannot be built apart — [`SeverityRationale`]
/// is one-to-one with [`Severity`]
/// (`sure_domain::finding::SeverityRationale::for_severity`), and
/// `FindingBuilder` refuses a pair that disagrees. Holding them as one value is
/// what stops this module handing a caller a `must_fix` whose rationale says
/// something else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gravity {
    severity: Severity,
    rationale: SeverityRationale,
}

/// Informational: something SURE noticed and cannot ask anyone to act on.
const INFORMATIONAL: Gravity = Gravity {
    severity: Severity::Note,
    rationale: SeverityRationale::Informational,
};

/// A non-blocking improvement: real, and not in anyone's way yet.
const NON_BLOCKING: Gravity = Gravity {
    severity: Severity::CanFixLater,
    rationale: SeverityRationale::NonBlockingImprovement,
};

/// A material reliability or quality risk.
const RELIABILITY_RISK: Gravity = Gravity {
    severity: Severity::ShouldFixFirst,
    rationale: SeverityRationale::ReliabilityOrQualityRisk,
};

/// Do not hand the project off while this stands.
const BLOCKS_HAND_OFF: Gravity = Gravity {
    severity: Severity::MustFix,
    rationale: SeverityRationale::BlocksHandOff,
};

impl Gravity {
    /// How serious the finding is.
    #[must_use]
    pub const fn severity(self) -> Severity {
        self.severity
    }

    /// Why that severity, in the domain's own terms.
    #[must_use]
    pub const fn rationale(self) -> SeverityRationale {
        self.rationale
    }
}

/// The severity and rationale for one detector finding.
///
/// See the module documentation for the rule this implements and for what it
/// does not claim. The gates are applied in order and each can only lower the
/// answer: a finding that fails any of them is informational.
#[must_use]
pub fn gravity_of(
    reason: &CheckReason,
    evidence_class: EvidenceClass,
    reach: Reach,
    gap: GapKind,
) -> Gravity {
    // A reason that names no file, component, command or line is not a claim
    // about anything, and a proposal built on one would be refused by
    // `PlanBuilder::propose` anyway. It cannot be acted on.
    if !reason.names_something() {
        return INFORMATIONAL;
    }

    // A finding a reader cannot go and look at is one they have to take on
    // trust. `CheckReason::anchor` returns `None` only for `ProjectWide`, which
    // is a true statement about a check and not a place; `is_checkable` refuses
    // an anchor whose location or locator is blank.
    match reason.anchor() {
        Some(anchor) if anchor.is_checkable() => {}
        _ => return INFORMATIONAL,
    }

    // A check that did not run establishes nothing, and one whose result would
    // be worth `unknown` establishes nothing either. SURE does not know what it
    // found, so it does not raise it.
    if evidence_class == EvidenceClass::Unknown {
        return INFORMATIONAL;
    }

    match (gap, reach) {
        // A statement about intent is a note even when it is about a project
        // that ships: the claim belongs to SURE's reading, and its anchor is a
        // requirement identifier rather than a file.
        (GapKind::DeclaredIntent, _) => INFORMATIONAL,
        // Tests, examples, documentation and mock fixtures cannot reach a user.
        // This is the gate that keeps a mock in `tests/` a note, which the
        // manifest requires of `benign-test-mocks` and which several fixtures
        // list under `forbidden_outcomes` as a `false_positive`.
        (_, Reach::NotProduction) => INFORMATIONAL,
        (GapKind::UnfinishedMarker, Reach::Production) => NON_BLOCKING,
        (GapKind::UnrealContent, Reach::Production) => RELIABILITY_RISK,
        (GapKind::SubstitutedAction, Reach::Production) => BLOCKS_HAND_OFF,
    }
}

/// Whether this severity means "too weak to show as material".
///
/// **The aggregator asks this instead of deciding for itself.** Before
/// `P7-T011`, `false_completion_aggregator::is_style_noise` was a conjunction of
/// three fields — `note`, not critical, `inference` — and every candidate from
/// every detector happened to satisfy it, so nothing a detector found was ever
/// shown as material. The rule above returns `note` exactly when a finding
/// failed one of its gates, so this predicate and the rule cannot disagree about
/// what "style noise" means: there is one table, and this reads its answer.
#[must_use]
pub const fn is_informational(severity: Severity) -> bool {
    matches!(severity, Severity::Note)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::severity::Severity;

    fn candidate(path: &str) -> CheckReason {
        CheckReason::CandidateFound {
            path: path.to_owned(),
            line: 7,
            context: "TODO: wire this up".to_owned(),
        }
    }

    /// Every pair this module can return, so the table below is checked over all
    /// of them rather than over the ones a test happened to construct.
    const EVERY_GRAVITY: &[Gravity] = &[
        INFORMATIONAL,
        NON_BLOCKING,
        RELIABILITY_RISK,
        BLOCKS_HAND_OFF,
    ];

    #[test]
    fn every_pair_this_module_returns_matches_the_domains_own_mapping() {
        // `FindingBuilder` refuses a finding whose rationale disagrees with its
        // severity, and `SeverityRationale::for_severity` is the domain's answer
        // to which one belongs with which. A pair written by hand here is a
        // second copy of that mapping, so the two are compared rather than
        // trusted -- and the four levels are asserted to be all of them, so a
        // level added to the domain cannot be quietly missing here.
        let mut seen: Vec<Severity> = Vec::new();
        for gravity in EVERY_GRAVITY {
            assert_eq!(
                SeverityRationale::for_severity(gravity.severity()),
                Some(gravity.rationale()),
                "{:?} is paired with the wrong rationale",
                gravity.severity()
            );
            assert!(!seen.contains(&gravity.severity()));
            seen.push(gravity.severity());
        }
        assert_eq!(
            seen.len(),
            Severity::ALL.len(),
            "a level is unreachable here"
        );
    }

    #[test]
    fn every_pair_this_module_returns_is_reachable_through_the_rule() {
        // A constant table nothing returns from is a claim the rule does not
        // make. Each of the four is produced by the rule below.
        let reachable = [
            gravity_of(
                &candidate("src/a.js"),
                EvidenceClass::Inference,
                Reach::Production,
                GapKind::SubstitutedAction,
            ),
            gravity_of(
                &candidate("src/a.js"),
                EvidenceClass::Inference,
                Reach::Production,
                GapKind::UnrealContent,
            ),
            gravity_of(
                &candidate("src/a.js"),
                EvidenceClass::Inference,
                Reach::Production,
                GapKind::UnfinishedMarker,
            ),
            gravity_of(
                &candidate("src/a.js"),
                EvidenceClass::Inference,
                Reach::Production,
                GapKind::DeclaredIntent,
            ),
        ];
        for gravity in EVERY_GRAVITY {
            assert!(
                reachable.contains(gravity),
                "{gravity:?} is in the table but the rule never returns it"
            );
        }
    }

    #[test]
    fn a_substituted_action_in_product_code_blocks_hand_off() {
        let gravity = gravity_of(
            &candidate("src/payments.js"),
            EvidenceClass::Inference,
            Reach::Production,
            GapKind::SubstitutedAction,
        );
        assert_eq!(gravity.severity(), Severity::MustFix);
        assert_eq!(gravity.rationale(), SeverityRationale::BlocksHandOff);
        assert!(gravity.severity().blocks_hand_off());
    }

    #[test]
    fn the_evidence_class_is_read_and_never_raised() {
        // The corpus requires `must_fix` *and* `inference` for the same finding
        // (`fixtures/adversarial/fake-payment/scenario.json`), so the rule must
        // not quietly turn a pattern guess into an observation to earn a
        // severity, and must not refuse the severity because the class is weak.
        for class in [
            EvidenceClass::ObservedFact,
            EvidenceClass::DeterministicCheck,
            EvidenceClass::ModelAssessment,
            EvidenceClass::Inference,
        ] {
            let gravity = gravity_of(
                &candidate("src/payments.js"),
                class,
                Reach::Production,
                GapKind::SubstitutedAction,
            );
            assert_eq!(
                gravity.severity(),
                Severity::MustFix,
                "{class:?} changed the answer, so severity is being decided by the class"
            );
        }

        // And the one class that does lower it, because a result worth nothing
        // supports nothing.
        assert_eq!(
            gravity_of(
                &candidate("src/payments.js"),
                EvidenceClass::Unknown,
                Reach::Production,
                GapKind::SubstitutedAction,
            )
            .severity(),
            Severity::Note
        );
    }

    #[test]
    fn a_finding_that_names_nothing_or_points_nowhere_is_a_note() {
        let empty = CheckReason::CandidateFound {
            path: "   ".to_owned(),
            line: 1,
            context: "something".to_owned(),
        };
        assert_eq!(
            gravity_of(
                &empty,
                EvidenceClass::Inference,
                Reach::Production,
                GapKind::SubstitutedAction
            )
            .severity(),
            Severity::Note,
            "a reason naming no file cannot support a must_fix"
        );

        // `ProjectWide` is a true statement about a check and is not a place.
        assert!(CheckReason::ProjectWide.anchor().is_none());
        assert_eq!(
            gravity_of(
                &CheckReason::ProjectWide,
                EvidenceClass::ObservedFact,
                Reach::Production,
                GapKind::SubstitutedAction
            )
            .severity(),
            Severity::Note
        );

        // A reason whose anchor carries a blank locator is not checkable either.
        let blank = CheckReason::FilePresent {
            path: "  ".to_owned(),
        };
        assert_eq!(
            gravity_of(
                &blank,
                EvidenceClass::Inference,
                Reach::Production,
                GapKind::SubstitutedAction
            )
            .severity(),
            Severity::Note
        );
    }

    #[test]
    fn what_cannot_reach_a_user_never_rises_above_a_note() {
        // The benign half of the corpus: a mock in `tests/` is not a production
        // finding whatever kind of gap it would be in product code.
        for context in [
            CandidateContext::Test,
            CandidateContext::Example,
            CandidateContext::Doc,
            CandidateContext::MockFixture,
        ] {
            for gap in [
                GapKind::UnfinishedMarker,
                GapKind::UnrealContent,
                GapKind::SubstitutedAction,
                GapKind::DeclaredIntent,
            ] {
                let gravity = gravity_of(
                    &candidate("tests/payments.rs"),
                    EvidenceClass::Inference,
                    Reach::from(context),
                    gap,
                );
                assert_eq!(
                    gravity.severity(),
                    Severity::Note,
                    "{context:?} with a {gap:?} reached {:?}",
                    gravity.severity()
                );
            }
        }

        // And `Product` is the one context that does not lower the answer, which
        // is what makes the loop above a claim about context rather than about a
        // rule that always answers `note`.
        assert_eq!(Reach::from(CandidateContext::Product), Reach::Production);
        assert_eq!(
            gravity_of(
                &candidate("src/payments.rs"),
                EvidenceClass::Inference,
                Reach::from(CandidateContext::Product),
                GapKind::SubstitutedAction
            )
            .severity(),
            Severity::MustFix
        );
    }

    #[test]
    fn a_statement_about_declared_intent_is_a_note_on_any_reach() {
        // `missing-user-intent` is `note` in the manifest, and its anchor is a
        // requirement identifier rather than a file. The gate is the gap kind
        // and not the reach, so declaring a production reach does not raise it.
        for reach in [Reach::Production, Reach::NotProduction] {
            let gravity = gravity_of(
                &CheckReason::CandidateFound {
                    path: "req_checkout_works".to_owned(),
                    line: 1,
                    context: "checkout works end to end".to_owned(),
                },
                EvidenceClass::Inference,
                reach,
                GapKind::DeclaredIntent,
            );
            assert_eq!(gravity.severity(), Severity::Note, "{reach:?}");
            assert_eq!(gravity.rationale(), SeverityRationale::Informational);
        }
    }

    #[test]
    fn the_three_gap_kinds_give_three_different_answers() {
        // A rule whose variants all answered the same thing would pass every
        // test above while deciding nothing, so the three are compared with one
        // another on identical inputs.
        let severities: Vec<Severity> = [
            GapKind::UnfinishedMarker,
            GapKind::UnrealContent,
            GapKind::SubstitutedAction,
        ]
        .into_iter()
        .map(|gap| {
            gravity_of(
                &candidate("src/a.js"),
                EvidenceClass::Inference,
                Reach::Production,
                gap,
            )
            .severity()
        })
        .collect();
        assert_eq!(
            severities,
            [
                Severity::CanFixLater,
                Severity::ShouldFixFirst,
                Severity::MustFix
            ]
        );
        let mut deduped = severities.clone();
        deduped.dedup();
        assert_eq!(
            deduped.len(),
            severities.len(),
            "two gaps agree, so one is dead"
        );
    }

    #[test]
    fn only_the_note_answer_counts_as_style_noise() {
        assert!(is_informational(Severity::Note));
        for severity in [
            Severity::CanFixLater,
            Severity::ShouldFixFirst,
            Severity::MustFix,
        ] {
            assert!(
                !is_informational(severity),
                "{severity:?} is above a note and must be shown as material"
            );
        }
    }
}

//! False-completion candidate aggregator.
//!
//! `P6-T009`'s acceptance: *deduplicates/grounds material candidates and
//! prioritizes user impact over style noise.*
//!
//! Several scanners each produce [`CheckProposal`] values for things that look
//! like incomplete or faked work. This module takes those raw candidates,
//! collapses duplicates that point at the same evidence anchor, drops proposals
//! that name nothing, and separates low-user-impact noise from the material a
//! report should show.
//!
//! # What "more serious" means
//!
//! When two proposals share an anchor, the single kept proposal is chosen by a
//! chain of comparisons, applied in order:
//!
//! 1. Higher [`Severity::rank`].
//! 2. `critical == true` over `critical == false`.
//! 3. Stronger [`EvidenceClass`] — `ObservedFact` beats `DeterministicCheck`,
//!    which beats `ModelAssessment`, which beats `Inference`, which beats
//!    `Unknown`.
//! 4. Fewer actions needed, on the theory that a narrower check is a more
//!    focused claim about the same place.
//! 5. Stable identifier order as a final tie-breaker.
//!
//! # What is not claimed
//!
//! This module does not decide whether a candidate is a real defect. It only
//! decides which of several proposals about the same place should be shown, and
//! which candidates are too weak to bother a user with.

use sure_domain::evidence::EvidenceClass;
use sure_domain::severity::Severity;

use crate::schedule::CheckProposal;

/// The result of aggregating false-completion candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedCandidates {
    material: Vec<CheckProposal>,
    style_noise: Vec<CheckProposal>,
    duplicates_dropped: usize,
}

impl AggregatedCandidates {
    /// The deduplicated, grounded candidates that should be shown to the user.
    #[must_use]
    pub fn material(&self) -> &[CheckProposal] {
        &self.material
    }

    /// Low-user-impact candidates that were deprioritized.
    #[must_use]
    pub fn style_noise(&self) -> &[CheckProposal] {
        &self.style_noise
    }

    /// How many proposals were removed because they shared an anchor with a kept
    /// proposal.
    #[must_use]
    pub fn duplicates_dropped(&self) -> usize {
        self.duplicates_dropped
    }

    /// Whether no candidates remain in either bucket.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.material.is_empty() && self.style_noise.is_empty()
    }

    /// Total number of candidates kept in both buckets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.material.len() + self.style_noise.len()
    }

    /// Every kept candidate, material first and then style noise.
    #[must_use]
    pub fn all_candidates(&self) -> Vec<&CheckProposal> {
        self.material
            .iter()
            .chain(self.style_noise.iter())
            .collect()
    }
}

/// Deduplicate and prioritize false-completion candidates.
///
/// Candidates are grouped by [`CheckReason::anchor`]. Proposals whose reason
/// names nothing are dropped entirely and do not count toward
/// [`AggregatedCandidates::duplicates_dropped`]. Within each anchor group the
/// most serious proposal is kept; the rest are counted as duplicates. After
/// deduplication, `Severity::Note` candidates that are not critical and carry
/// `EvidenceClass::Inference` are moved to [`AggregatedCandidates::style_noise`];
/// everything else becomes [`AggregatedCandidates::material`], ordered by
/// severity worst-first, then critical-first, then identifier.
pub fn aggregate(candidates: impl IntoIterator<Item = CheckProposal>) -> AggregatedCandidates {
    let mut anchored: Vec<(sure_domain::evidence::EvidenceAnchor, Vec<CheckProposal>)> = Vec::new();
    let mut unanchored: Vec<CheckProposal> = Vec::new();

    for proposal in candidates {
        if !proposal.reason().names_something() {
            continue;
        }
        match proposal.reason().anchor() {
            Some(anchor) => {
                if let Some((_, group)) = anchored.iter_mut().find(|(a, _)| a == &anchor) {
                    group.push(proposal);
                } else {
                    anchored.push((anchor, vec![proposal]));
                }
            }
            None => unanchored.push(proposal),
        }
    }

    let mut kept: Vec<CheckProposal> = Vec::new();
    let mut duplicates_dropped = 0_usize;

    for (_, group) in anchored {
        duplicates_dropped += group.len().saturating_sub(1);
        if let Some(winner) = group.into_iter().reduce(|current, proposal| {
            if seriousness_key(&current) < seriousness_key(&proposal) {
                proposal
            } else {
                current
            }
        }) {
            kept.push(winner);
        }
    }

    kept.extend(unanchored);

    let mut material = Vec::new();
    let mut style_noise = Vec::new();
    for proposal in kept {
        if is_style_noise(&proposal) {
            style_noise.push(proposal);
        } else {
            material.push(proposal);
        }
    }

    material.sort_by(material_order);
    style_noise.sort_by(id_order);

    AggregatedCandidates {
        material,
        style_noise,
        duplicates_dropped,
    }
}

/// Whether a candidate is too weak to show as material.
fn is_style_noise(proposal: &CheckProposal) -> bool {
    proposal.severity() == Severity::Note
        && !proposal.critical()
        && proposal.evidence_class() == EvidenceClass::Inference
}

/// A comparable key where "more serious" sorts greater.
fn seriousness_key(
    proposal: &CheckProposal,
) -> (
    u8,
    bool,
    std::cmp::Reverse<u8>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<&str>,
) {
    (
        proposal.severity().rank(),
        proposal.critical(),
        std::cmp::Reverse(evidence_class_weight(proposal.evidence_class())),
        std::cmp::Reverse(proposal.requirements().actions().len()),
        std::cmp::Reverse(proposal.id().as_str()),
    )
}

/// Evidence-class weight where a smaller number means a stronger class.
const fn evidence_class_weight(class: EvidenceClass) -> u8 {
    match class {
        EvidenceClass::ObservedFact => 0,
        EvidenceClass::DeterministicCheck => 1,
        EvidenceClass::ModelAssessment => 2,
        EvidenceClass::Inference => 3,
        EvidenceClass::Unknown => 4,
    }
}

/// Order for [`AggregatedCandidates::material`]: worst first, critical first,
/// then identifier.
fn material_order(left: &CheckProposal, right: &CheckProposal) -> std::cmp::Ordering {
    left.severity()
        .rank()
        .cmp(&right.severity().rank())
        .reverse()
        .then_with(|| right.critical().cmp(&left.critical()))
        .then_with(|| left.id().as_str().cmp(right.id().as_str()))
}

/// Stable identifier order for style noise.
fn id_order(left: &CheckProposal, right: &CheckProposal) -> std::cmp::Ordering {
    left.id().as_str().cmp(right.id().as_str())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::evidence::EvidenceClass;
    use sure_domain::execution::ActionKind;
    use sure_domain::ids::CheckId;
    use sure_domain::severity::Severity;

    use crate::redact::escape_control_characters;
    use crate::schedule::CheckReason;

    fn proposal(
        id: &str,
        title: &str,
        severity: Severity,
        critical: bool,
        evidence_class: EvidenceClass,
        reason: CheckReason,
        actions: &[ActionKind],
    ) -> CheckProposal {
        CheckProposal::new(
            CheckId::parse(id).expect("test id is well-formed"),
            title,
            severity,
            critical,
            evidence_class,
            reason,
            actions,
        )
    }

    fn candidate_reason(path: &str, line: usize, context: &str) -> CheckReason {
        CheckReason::CandidateFound {
            path: path.to_owned(),
            line,
            context: context.to_owned(),
        }
    }

    #[test]
    fn identical_anchor_duplicates_collapse_to_the_most_severe() {
        let reason = candidate_reason("src/lib.rs", 10, "TODO: fix");
        let weak = proposal(
            "chk_a",
            "weak",
            Severity::Note,
            false,
            EvidenceClass::Inference,
            reason.clone(),
            &[ActionKind::ReadFile],
        );
        let strong = proposal(
            "chk_b",
            "strong",
            Severity::MustFix,
            true,
            EvidenceClass::ObservedFact,
            reason,
            &[ActionKind::ReadFile],
        );

        let aggregated = aggregate([weak, strong]);
        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated.material().len(), 1);
        assert_eq!(aggregated.style_noise().len(), 0);
        assert_eq!(aggregated.duplicates_dropped(), 1);
        assert_eq!(aggregated.material()[0].title(), "strong");
        assert_eq!(aggregated.material()[0].severity(), Severity::MustFix);
    }

    #[test]
    fn different_anchors_are_kept_separately() {
        let first = candidate_reason("src/a.rs", 1, "TODO");
        let second = candidate_reason("src/b.rs", 2, "FIXME");
        let proposals = [
            proposal(
                "chk_a",
                "a",
                Severity::Note,
                false,
                EvidenceClass::Inference,
                first,
                &[ActionKind::ReadFile],
            ),
            proposal(
                "chk_b",
                "b",
                Severity::Note,
                false,
                EvidenceClass::Inference,
                second,
                &[ActionKind::ReadFile],
            ),
        ];

        let aggregated = aggregate(proposals);
        assert_eq!(aggregated.len(), 2);
        assert_eq!(aggregated.material().len(), 0);
        assert_eq!(aggregated.style_noise().len(), 2);
        assert_eq!(aggregated.duplicates_dropped(), 0);
    }

    #[test]
    fn non_names_something_proposals_are_dropped_and_not_counted_as_noise() {
        let empty_reason = CheckReason::FilePresent {
            path: String::new(),
        };
        let proposals = [proposal(
            "chk_a",
            "empty reason",
            Severity::Note,
            false,
            EvidenceClass::Inference,
            empty_reason,
            &[ActionKind::ReadFile],
        )];

        let aggregated = aggregate(proposals);
        assert!(aggregated.is_empty());
        assert_eq!(aggregated.len(), 0);
        assert_eq!(aggregated.style_noise().len(), 0);
        assert_eq!(aggregated.duplicates_dropped(), 0);
    }

    #[test]
    fn low_impact_note_inference_candidates_move_to_style_noise() {
        let proposals = [proposal(
            "chk_a",
            "noise",
            Severity::Note,
            false,
            EvidenceClass::Inference,
            candidate_reason("src/lib.rs", 5, "stub"),
            &[ActionKind::ReadFile],
        )];

        let aggregated = aggregate(proposals);
        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated.material().len(), 0);
        assert_eq!(aggregated.style_noise().len(), 1);
        assert_eq!(aggregated.style_noise()[0].title(), "noise");
    }

    #[test]
    fn note_inference_duplicate_of_serious_anchor_is_dropped_for_the_serious_one() {
        let reason = candidate_reason("src/pay.rs", 7, "tok_visa");
        let noise = proposal(
            "chk_a",
            "inference noise",
            Severity::Note,
            false,
            EvidenceClass::Inference,
            reason.clone(),
            &[ActionKind::ReadFile],
        );
        let grounded = proposal(
            "chk_b",
            "observed fake payment",
            Severity::MustFix,
            true,
            EvidenceClass::ObservedFact,
            reason,
            &[ActionKind::ReadFile],
        );

        let aggregated = aggregate([noise, grounded]);
        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated.material().len(), 1);
        assert_eq!(aggregated.style_noise().len(), 0);
        assert_eq!(aggregated.duplicates_dropped(), 1);
        assert_eq!(aggregated.material()[0].title(), "observed fake payment");
    }

    #[test]
    fn escaping_is_preserved_in_aggregated_titles() {
        let raw = "project has a\nline break";
        let escaped = escape_control_characters(raw);
        assert!(!escaped.contains('\n'));

        let proposals = [proposal(
            "chk_a",
            &escaped,
            Severity::Note,
            false,
            EvidenceClass::Inference,
            candidate_reason("src/lib.rs", 1, "placeholder"),
            &[ActionKind::ReadFile],
        )];

        let aggregated = aggregate(proposals);
        assert_eq!(aggregated.style_noise()[0].title(), escaped);
        assert!(!aggregated.style_noise()[0].title().contains('\n'));
    }

    #[test]
    fn ordering_is_deterministic() {
        let reasons = [
            candidate_reason("src/z.rs", 1, "z"),
            candidate_reason("src/m.rs", 2, "m"),
            candidate_reason("src/a.rs", 3, "a"),
        ];
        let proposals = [
            proposal(
                "chk_z",
                "z",
                Severity::Note,
                false,
                EvidenceClass::Inference,
                reasons[0].clone(),
                &[ActionKind::ReadFile],
            ),
            proposal(
                "chk_m",
                "m",
                Severity::CanFixLater,
                false,
                EvidenceClass::Inference,
                reasons[1].clone(),
                &[ActionKind::ReadFile],
            ),
            proposal(
                "chk_a",
                "a",
                Severity::MustFix,
                true,
                EvidenceClass::ObservedFact,
                reasons[2].clone(),
                &[ActionKind::ReadFile],
            ),
        ];

        let proposals_for_reorder = proposals.clone();
        let aggregated = aggregate(proposals);
        let titles: Vec<&str> = aggregated.material().iter().map(|p| p.title()).collect();
        assert_eq!(titles, ["a", "m"]);

        let noise_titles: Vec<&str> = aggregated.style_noise().iter().map(|p| p.title()).collect();
        assert_eq!(noise_titles, ["z"]);

        // Run again in a different input order and expect the same buckets.
        let reordered = aggregate([
            proposals_for_reorder[2].clone(),
            proposals_for_reorder[0].clone(),
            proposals_for_reorder[1].clone(),
        ]);
        assert_eq!(
            reordered
                .material()
                .iter()
                .map(|p| p.title())
                .collect::<Vec<_>>(),
            titles
        );
        assert_eq!(
            reordered
                .style_noise()
                .iter()
                .map(|p| p.title())
                .collect::<Vec<_>>(),
            noise_titles
        );
    }
}

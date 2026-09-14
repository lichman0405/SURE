//! Finding severity.
//!
//! The four levels are frozen by `docs/product/UX_AND_LANGUAGE.md` and by
//! `schemas/finding.schema.json`, in both wording and wire name. Nothing in
//! SURE may invent a fifth level or a numeric confidence score: uncertainty is
//! expressed as `cannot_confirm`, never as a percentage.

use serde::{Deserialize, Serialize};

/// How much a finding matters to the person receiving the report.
///
/// Greater means more serious, matching [`Severity::rank`]. The ordering is
/// written by hand so that "sort by severity" always means "worst first" and
/// cannot silently invert if the variants are ever reordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Do not recommend publishing or handing this off.
    #[serde(rename = "must_fix")]
    MustFix,
    /// A material reliability or quality risk.
    #[serde(rename = "should_fix_first")]
    ShouldFixFirst,
    /// A non-blocking improvement.
    #[serde(rename = "can_fix_later")]
    CanFixLater,
    /// Informational.
    Note,
}

impl Severity {
    /// Every level, most serious first.
    pub const ALL: [Self; 4] = [
        Self::MustFix,
        Self::ShouldFixFirst,
        Self::CanFixLater,
        Self::Note,
    ];

    /// The stable wire name used by the JSON schemas.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MustFix => "must_fix",
            Self::ShouldFixFirst => "should_fix_first",
            Self::CanFixLater => "can_fix_later",
            Self::Note => "note",
        }
    }

    /// The exact user-facing label from the product language rules.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MustFix => "Must fix",
            Self::ShouldFixFirst => "Should fix first",
            Self::CanFixLater => "Can fix later",
            Self::Note => "Note",
        }
    }

    /// Whether this level, on its own, forbids hand-off.
    #[must_use]
    pub const fn blocks_hand_off(self) -> bool {
        matches!(self, Self::MustFix)
    }

    /// Ranking where a larger number is more serious.
    ///
    /// Used to break ties when several findings compete for the report headline.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Note => 0,
            Self::CanFixLater => 1,
            Self::ShouldFixFirst => 2,
            Self::MustFix => 3,
        }
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_match_the_frozen_labels() {
        assert_eq!(Severity::MustFix.as_str(), "must_fix");
        assert_eq!(Severity::ShouldFixFirst.as_str(), "should_fix_first");
        assert_eq!(Severity::CanFixLater.as_str(), "can_fix_later");
        assert_eq!(Severity::Note.as_str(), "note");
        for severity in Severity::ALL {
            let json = serde_json::to_string(&severity).expect("serialize");
            assert_eq!(json, format!("\"{}\"", severity.as_str()));
        }
    }

    #[test]
    fn labels_match_the_product_language_rules() {
        assert_eq!(Severity::MustFix.label(), "Must fix");
        assert_eq!(Severity::ShouldFixFirst.label(), "Should fix first");
        assert_eq!(Severity::CanFixLater.label(), "Can fix later");
        assert_eq!(Severity::Note.label(), "Note");
    }

    #[test]
    fn greater_always_means_more_serious() {
        // `Severity::ALL` is listed most serious first, so it is a descending
        // run under this ordering. Sorting ascending therefore puts the least
        // serious finding first and `Reverse` puts the worst first.
        for pair in Severity::ALL.windows(2) {
            assert!(
                pair[0] > pair[1],
                "{:?} should be more serious than {:?}",
                pair[0],
                pair[1]
            );
            assert!(pair[0].rank() > pair[1].rank());
        }
        assert_eq!(
            Severity::ALL.into_iter().max(),
            Some(Severity::MustFix),
            "max must be the most serious level"
        );
    }

    #[test]
    fn only_must_fix_blocks_hand_off_by_itself() {
        assert!(Severity::MustFix.blocks_hand_off());
        for severity in [
            Severity::ShouldFixFirst,
            Severity::CanFixLater,
            Severity::Note,
        ] {
            assert!(!severity.blocks_hand_off());
        }
    }
}

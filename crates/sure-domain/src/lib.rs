#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    MustFix,
    ShouldFixFirst,
    CanFixLater,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimAssessment {
    Confirmed,
    Contradicted,
    CannotConfirm,
    NotCheckable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    ObservedFact,
    DeterministicCheck,
    ModelAssessment,
    Inference,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_confirm_is_distinct_from_contradicted() {
        assert_ne!(
            ClaimAssessment::CannotConfirm,
            ClaimAssessment::Contradicted
        );
    }
}

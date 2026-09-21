//! Plain-language AI-claim report section.
//!
//! Renders the result of checking agent completion claims against recorded
//! harness events. Every claim is shown with its assessment and a plain
//! explanation, with attacker-controlled text escaped before embedding.

use sure_domain::evidence::ClaimAssessment;
use sure_domain::vocabulary::Claim;

use crate::redact::escape_control_characters;

/// Render a plain-language section describing checked agent claims.
///
/// Returns an empty string when there are no claims to report, so callers can
/// simply append the result without extra guarding.
#[must_use]
pub fn render_claim_section(claims: &[Claim]) -> String {
    let entries = render_claim_entries(claims);
    if entries.is_empty() {
        return String::new();
    }

    let mut out = String::from("AI claims\n");
    out.push_str("---------\n");
    out.push_str(&entries);
    out
}

/// Render the claim list without a section header.
///
/// Useful for reports that supply their own heading (Markdown/HTML).
#[must_use]
pub fn render_claim_entries(claims: &[Claim]) -> String {
    if claims.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for claim in claims {
        out.push_str("- ");
        out.push_str(escape_control_characters(&claim.claim_text).as_str());
        out.push('\n');
        out.push_str("  ");
        out.push_str(claim.assessment.label());
        out.push_str(": ");
        out.push_str(escape_control_characters(&claim_reason(claim)).as_str());
        out.push('\n');
    }
    out
}

/// Provide a plain-language reason for a claim assessment.
///
/// If the claim carries its own reason, use it; otherwise fall back to a safe
/// default that explains the assessment label without leaking control
/// characters.
fn claim_reason(claim: &Claim) -> String {
    if !claim.reason.is_empty() {
        return claim.reason.clone();
    }

    match claim.assessment {
        ClaimAssessment::Confirmed => String::from("A recorded harness event supports this claim."),
        ClaimAssessment::Contradicted => {
            String::from("A recorded harness event contradicts this claim.")
        }
        ClaimAssessment::CannotConfirm => {
            String::from("SURE has no recorded event that supports or contradicts this claim.")
        }
        ClaimAssessment::NotCheckable => {
            if claim.claim_type.is_empty() {
                String::from("SURE does not know how to check this kind of claim.")
            } else {
                format!(
                    "SURE does not know how to check a '{}' claim.",
                    escape_control_characters(&claim.claim_type)
                )
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sure_domain::ids::ClaimId;

    fn claim(assessment: ClaimAssessment, text: &str, reason: &str) -> Claim {
        Claim {
            id: ClaimId::generate(),
            claim_text: text.to_owned(),
            claim_type: "test_ran".to_owned(),
            assessment,
            reason: reason.to_owned(),
            evidence: Vec::new(),
            session: None,
        }
    }

    #[test]
    fn empty_claim_list_renders_nothing() {
        assert!(render_claim_section(&[]).is_empty());
    }

    #[test]
    fn confirmed_claim_is_explained_plainly() {
        let claims = vec![claim(
            ClaimAssessment::Confirmed,
            "I ran the tests",
            "A recorded harness event supports this claim.",
        )];
        let section = render_claim_section(&claims);
        assert!(section.contains("AI claims"), "{section}");
        assert!(section.contains("I ran the tests"), "{section}");
        assert!(section.contains("Confirmed"), "{section}");
        assert!(
            section.contains("A recorded harness event supports this claim."),
            "{section}"
        );
    }

    #[test]
    fn cannot_confirm_claim_is_explained_plainly() {
        let claims = vec![claim(
            ClaimAssessment::CannotConfirm,
            "I fixed the bug",
            "SURE has no recorded event that supports this claim.",
        )];
        let section = render_claim_section(&claims);
        assert!(section.contains("Cannot confirm"), "{section}");
        assert!(
            section.contains("SURE has no recorded event that supports this claim."),
            "{section}"
        );
    }

    #[test]
    fn attacker_controlled_claim_text_is_escaped() {
        let claims = vec![claim(
            ClaimAssessment::Confirmed,
            "I ran the tests\nsecret",
            "ok",
        )];
        let section = render_claim_section(&claims);
        assert!(!section.contains("I ran the tests\nsecret"), "{section:?}");
        assert!(section.contains("I ran the tests\\nsecret"), "{section}");
    }

    #[test]
    fn missing_reason_falls_back_to_safe_default() {
        let mut c = claim(ClaimAssessment::NotCheckable, "weird claim", "");
        c.claim_type = String::new();
        let section = render_claim_section(&[c]);
        assert!(
            section.contains("SURE does not know how to check this kind of claim."),
            "{section}"
        );
    }
}

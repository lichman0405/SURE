#![forbid(unsafe_code)]
//! SURE check engine.
//!
//! This crate will hold the discovery, planning, checking and reporting engine.
//! It currently re-exports the frozen domain vocabulary so that callers have a
//! single import path, and so that the crate boundary is exercised from the
//! first commit rather than introduced later.

pub use sure_domain::{
    PRODUCT_EXPANSION, PRODUCT_NAME, PRODUCT_PROMISE, capability, evidence, execution, ids, intent,
    severity, status, vocabulary,
};

/// The integration protocol version this core build speaks.
///
/// Re-exported rather than restated, so a core that has not been taught a newer
/// protocol cannot claim to speak it. The handshake that uses this lands in
/// P1-T010; declaring it here is what makes the `sure-core -> sure-protocol`
/// boundary a real edge from the first commit rather than a manifest line.
pub const PROTOCOL_VERSION: u32 = sure_protocol::PROTOCOL_VERSION;

/// The product name.
pub const NAME: &str = PRODUCT_NAME;

/// The product promise shown to users.
pub const PROMISE: &str = PRODUCT_PROMISE;

/// The version string reported by the CLI.
#[must_use]
pub fn version_string() -> String {
    sure_domain::version_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_core_reexports_the_frozen_vocabulary() {
        assert_eq!(NAME, "SURE");
        assert_eq!(PROMISE, "AI says it's done. Be SURE.");
        assert_eq!(
            status::NO_TRUSTED_INTENT_LIMITATION,
            sure_domain::status::NO_TRUSTED_INTENT_LIMITATION,
            "the re-export must be the same constant, not a copy that can drift"
        );
        assert!(status::NO_TRUSTED_INTENT_LIMITATION.contains("cannot confirm"));
    }

    #[test]
    fn a_critical_skipped_check_cannot_aggregate_to_green_through_the_core_reexport() {
        let result = status::CheckResult::not_run(
            ids::CheckId::generate(),
            "run the test suite",
            severity::Severity::MustFix,
            true,
            status::NotCheckedReason::ExecutionNotAuthorized,
        );
        assert!(!status::aggregate(&[result]).is_green());
    }
}

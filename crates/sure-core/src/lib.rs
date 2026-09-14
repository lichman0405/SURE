#![forbid(unsafe_code)]
//! SURE check engine.
//!
//! This crate will hold the discovery, planning, checking and reporting engine.
//! It currently re-exports the frozen domain vocabulary so that callers have a
//! single import path, and so that the crate boundary is exercised from the
//! first commit rather than introduced later.

pub mod components;
pub mod config;
pub mod diagnostics;
pub mod discover;
pub mod doctor;
pub mod fingerprint;
pub mod paths;
pub mod project_intent;
pub mod redact;
pub mod scan;
pub mod store;

pub use sure_domain::{
    PRODUCT_EXPANSION, PRODUCT_NAME, PRODUCT_PROMISE, capability, evidence, execution, ids, intent,
    severity, status, vocabulary,
};

/// The integration protocol version this core build speaks.
///
/// Re-exported rather than restated, so a core that has not been taught a newer
/// protocol cannot claim to speak it. Declaring it here is what makes the
/// `sure-core -> sure-protocol` boundary a real edge rather than a manifest
/// line; [`negotiate`] is the rule that uses it.
pub const PROTOCOL_VERSION: u32 = sure_protocol::PROTOCOL_VERSION;

/// The product name.
pub const NAME: &str = PRODUCT_NAME;

/// The product promise shown to users.
pub const PROMISE: &str = PRODUCT_PROMISE;

/// The version rule, re-exported from the crate that owns the wire contract.
///
/// The CLI answers a caller that asks whether the two can talk, and the event
/// reader refuses a document in a version this build does not speak. They are
/// the same rule, so they are the same function rather than two comparisons that
/// happen to agree today. Re-exported rather than reached directly because
/// `sure-cli` has one edge into the engine, not two (ADR 0001, and the note in
/// its manifest).
pub use sure_protocol::handshake::{Handshake, negotiate};

/// The version number of this build, without the product name.
///
/// Re-exported so that a report naming the build and a `sure version` naming it
/// cannot disagree: `sure_core::VERSION` is the same constant
/// [`version_string`] is built from, not a second copy of it.
pub const VERSION: &str = sure_domain::VERSION;

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
            ids::FingerprintId::generate(),
        );
        assert!(!status::aggregate(&[result]).is_green());
    }
}

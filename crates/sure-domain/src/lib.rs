#![forbid(unsafe_code)]
//! SURE domain vocabulary and frozen semantics.
//!
//! This crate is the single source of truth for what SURE's words mean. It has
//! no dependencies on storage, processes or integrations, so every other crate
//! can depend on it without inheriting policy decisions.
//!
//! Three rules are enforced here rather than merely documented:
//!
//! 1. **No false green.** [`status::aggregate`] cannot return a green result when
//!    a critical check failed, was skipped, errored or is unknown.
//! 2. **No overclaimed requirements.** [`intent::IntentSource`] will not let an
//!    inference or an agent claim become a user requirement, and
//!    [`intent::may_claim_full_fulfilment`] is the only gate for the sentence
//!    "everything you asked for is done".
//! 3. **No unlabelled evidence.** Every [`evidence::Evidence`] carries an
//!    [`evidence::EvidenceClass`], and only observed facts and deterministic
//!    checks can support a `must_fix` finding on their own.

pub mod capability;
pub mod evidence;
pub mod execution;
pub mod ids;
pub mod intent;
pub mod severity;
pub mod status;
pub mod vocabulary;

/// The product name.
pub const PRODUCT_NAME: &str = "SURE";

/// What the name stands for.
pub const PRODUCT_EXPANSION: &str = "Software Understanding & Reality Evaluation";

/// The product promise shown to users.
pub const PRODUCT_PROMISE: &str = "AI says it's done. Be SURE.";

/// Version of the frozen domain semantics in this crate.
///
/// Bumping this is a deliberate act: it means the meaning of a stored verdict,
/// finding or evidence record changed, and stored records may need reinterpretation.
pub const DOMAIN_SEMANTICS_VERSION: u32 = 1;

/// The version string reported by the CLI.
#[must_use]
pub fn version_string() -> String {
    format!("{PRODUCT_NAME} {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_product_names_are_the_frozen_ones() {
        assert_eq!(PRODUCT_NAME, "SURE");
        assert_eq!(
            PRODUCT_EXPANSION,
            "Software Understanding & Reality Evaluation"
        );
        assert_eq!(PRODUCT_PROMISE, "AI says it's done. Be SURE.");
    }

    #[test]
    fn version_is_not_empty() {
        assert!(!version_string().is_empty());
    }
}

//! Protection decision adapter for harness pre-action hooks.
//!
//! Maps harness tool requests to [`ExecutionDecision`] using the existing
//! domain machinery rather than inventing a new rule engine.
//!
//! P11-T006: Cursor protection where supported.

use serde::{Deserialize, Serialize};
use sure_domain::execution::{
    ActionKind, ExecutionDecision, ExecutionMode, ExecutionPermissions, decide,
};

/// What the protection adapter decided about a tool request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtectionDecisionKind {
    /// The tool may proceed.
    Allow,
    /// The tool may proceed, but SURE advises caution.
    Warn,
    /// The tool should not proceed.
    Block,
}

impl ProtectionDecisionKind {
    /// The stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Block => "block",
        }
    }
}

/// A protection decision with an optional human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionDecision {
    /// The decision itself.
    pub decision: ProtectionDecisionKind,
    /// Why the decision was made, when it is not obvious.
    pub reason: Option<String>,
}

impl ProtectionDecision {
    /// Allow with no reason needed.
    #[must_use]
    pub fn allow() -> Self {
        Self {
            decision: ProtectionDecisionKind::Allow,
            reason: None,
        }
    }

    /// Warn with a reason.
    #[must_use]
    pub fn warn(reason: impl Into<String>) -> Self {
        Self {
            decision: ProtectionDecisionKind::Warn,
            reason: Some(reason.into()),
        }
    }

    /// Block with a reason.
    #[must_use]
    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            decision: ProtectionDecisionKind::Block,
            reason: Some(reason.into()),
        }
    }
}

/// Map a Cursor tool name to the [`ActionKind`] the domain understands.
fn cursor_tool_to_action_kind(tool: &str) -> ActionKind {
    match tool {
        "Shell" => ActionKind::ArbitraryCommand,
        "Read" => ActionKind::ReadFile,
        "Write" => ActionKind::WriteProjectFile,
        "Delete" => ActionKind::DeleteProjectFile,
        _ => ActionKind::ArbitraryCommand,
    }
}

/// Decide whether a Cursor `preToolUse` request should be allowed.
///
/// Uses the existing domain [`decide`] function with the current execution
/// mode and permissions. No new rule engine is invented.
///
/// # Capability tier honesty
///
/// Cursor remains **Observed** (Tier 1). The hook manifest does not confirm
/// Cursor interprets the response, so protection is advisory/warn-only from
/// the integration's point of view. The decision still uses the real mode and
/// permissions so that the answer is truthful about what SURE would do.
#[must_use]
pub fn decide_cursor_tool(
    tool: &str,
    mode: ExecutionMode,
    permissions: &ExecutionPermissions,
) -> ProtectionDecision {
    let action_kind = cursor_tool_to_action_kind(tool);

    match decide(action_kind, mode, permissions) {
        ExecutionDecision::Allowed => ProtectionDecision::allow(),
        ExecutionDecision::NeedsConsent => {
            ProtectionDecision::warn("This action needs explicit approval before it can run.")
        }
        ExecutionDecision::Denied => {
            ProtectionDecision::block("The current execution mode does not permit this action.")
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn read_file_is_allowed_in_inspect_only() {
        let decision = decide_cursor_tool(
            "Read",
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
        assert!(decision.reason.is_none());
    }

    #[test]
    fn shell_is_blocked_in_inspect_only() {
        let decision = decide_cursor_tool(
            "Shell",
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn delete_is_blocked_in_inspect_only() {
        let decision = decide_cursor_tool(
            "Delete",
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn write_is_blocked_in_inspect_only() {
        let decision = decide_cursor_tool(
            "Write",
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn delete_is_allowed_with_write_permission() {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.write_project = true;
        let decision = decide_cursor_tool("Delete", ExecutionMode::HostConfirmed, &permissions);
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn write_is_allowed_with_write_permission() {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.write_project = true;
        let decision = decide_cursor_tool("Write", ExecutionMode::HostConfirmed, &permissions);
        assert_eq!(decision.decision, ProtectionDecisionKind::Allow);
    }

    #[test]
    fn shell_needs_consent_even_with_run_project_code() {
        let mut permissions = ExecutionPermissions::inspect_only();
        permissions.run_project_code = true;
        let decision = decide_cursor_tool("Shell", ExecutionMode::HostConfirmed, &permissions);
        // ArbitraryCommand always returns NeedsConsent when the permission
        // is granted, which maps to Warn.
        assert_eq!(decision.decision, ProtectionDecisionKind::Warn);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn unknown_tool_is_treated_as_arbitrary_command() {
        let decision = decide_cursor_tool(
            "UnknownTool",
            ExecutionMode::InspectOnly,
            &ExecutionPermissions::inspect_only(),
        );
        assert_eq!(decision.decision, ProtectionDecisionKind::Block);
    }
}

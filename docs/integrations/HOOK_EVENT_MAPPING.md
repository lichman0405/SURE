# Hook event mapping

The exact public schemas are validated during implementation. This document defines semantic mapping, not a permanent vendor schema claim.

| SURE semantic event | Claude Code candidate | Cursor candidate |
|---|---|---|
| session.started | SessionStart | sessionStart |
| action.requested | PreToolUse | preToolUse / beforeShellExecution / beforeReadFile |
| action.completed | PostToolUse | postToolUse / afterShellExecution / afterFileEdit |
| action.failed | tool failure where exposed | postToolUseFailure |
| session.stopping | Stop | stop/sessionEnd where exposed |
| protection.decision | PreToolUse response | pre-tool/before-shell response |

Adapters preserve raw event source/version metadata so later schema evolution is diagnosable.

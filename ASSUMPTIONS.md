# Frozen implementation assumptions

These assumptions are intentionally resolved so Claude Code does not repeatedly ask the owner routine product questions.

- Primary users: individual vibe coders / AI-heavy builders; secondary users: small fast-moving teams.
- Core is open source under Apache-2.0.
- Team/cloud commercial features are future scope and must not distort v0.1.
- **Windows 11 x64 is the primary local development environment.**
- Rust core remains cross-platform; Windows/macOS/Linux CI is required.
- Windows uses native MSVC Rust, not WSL as the canonical build path.
- PowerShell 7 is the primary bootstrap/release scripting shell on Windows; Git Bash is optional.
- Claude Code is the first-class integration.
- Cursor is the second first-class integration, implemented as a Cursor Plugin/hooks package, not a bespoke IDE replacement.
- Codex gets a portable Agent Plugin/skill path plus any current native integration surface that can be verified during implementation.
- Copilot remains adapter-ready and is not a v0.1 blocker.
- No hosted LLM service is provided by SURE.
- Model-assisted analysis is optional. Deterministic/no-model mode must remain useful.
- Local-first is the privacy default.
- Full prompt/response/terminal capture is opt-in.
- SURE is allowed to answer `Cannot confirm`.
- SURE must not silently execute dependency installs, arbitrary README commands or unknown project code without an execution-trust decision.
- Project-level configuration cannot grant itself more execution/privacy/protection authority than user/organization policy.
- Authoritative evidence/history lives outside the checked working tree.
- No persistent local daemon is required for v0.1. Hooks invoke a short-lived CLI/core path; an on-demand daemon can be introduced later only if evidence shows it is necessary.

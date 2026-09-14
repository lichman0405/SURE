# Product/integration research snapshot — 2026-09-14

Implementation should re-check current public documentation before depending on an external integration schema.

Key references used for the bootstrap:

- Rust latest stable/release notes: https://blog.rust-lang.org/releases/latest/
- Node.js release status: https://nodejs.org/en/about/previous-releases
- Claude Code plugin/hooks documentation: https://code.claude.com/docs/
- Claude Code plugin reference source: https://github.com/anthropics/claude-code
- Cursor hooks: https://cursor.com/docs/hooks
- Cursor third-party hooks: https://cursor.com/docs/reference/third-party-hooks
- Cursor plugins: https://cursor.com/docs/plugins
- Cursor plugin reference: https://cursor.com/docs/reference/plugins
- OpenAI developer/Codex/plugin resources: https://developers.openai.com/
- OPA: https://www.openpolicyagent.org/
- Sigstore: https://www.sigstore.dev/
- SLSA: https://slsa.dev/

Do not hard-code a 2026 integration assumption forever. External adapter tasks must validate the currently installed/public schema during implementation.

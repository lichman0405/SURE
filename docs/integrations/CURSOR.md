# Cursor integration

Priority: first-class.

v0.1 starts with a **Cursor Plugin** rather than a custom VS Code/TypeScript extension.

Current Cursor plugin format supports commands and hooks under `.cursor-plugin/plugin.json`; Cursor hooks can observe/control agent activity, and Cursor can also load Claude Code third-party hooks.

SURE keeps an explicit Cursor package so it can:
- use Cursor-native hook names and event payloads;
- report its true capability tier;
- expose simple SURE commands;
- keep the core engine outside Cursor.

Package root: `integrations/cursor/`.

A TypeScript extension may be introduced only if a specific required capability cannot be delivered with plugins/hooks. Such a change requires an ADR and must not duplicate check logic.

Windows launcher/PATH handling is a release requirement. The integration must work when Cursor is launched from the Start menu and must not depend on a developer shell being open. The installer should resolve/render the SURE executable path or otherwise establish a reliable per-user invocation. macOS GUI PATH remains secondary cross-platform coverage.

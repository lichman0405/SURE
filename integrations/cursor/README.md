# SURE Cursor Plugin

SURE uses Cursor Plugin + hooks/commands rather than starting with a heavy VS Code/TypeScript extension.

The bootstrap hook template is Windows-first and invokes a thin PowerShell launcher. Cursor hooks communicate through stdio JSON. Before release, tasks must validate exact current Cursor event/input/output semantics and produce/render appropriate launchers for Windows plus secondary macOS/Linux packaging.

Cursor can also load Claude Code third-party hooks, but SURE keeps an explicit Cursor package so it can use native Cursor events/capability reporting.

If a capability truly requires an extension API, record an ADR and keep core checking logic in Rust.

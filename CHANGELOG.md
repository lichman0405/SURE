# Changelog

## v0.3.1 — Windows environment checker hotfix

- Fixed PowerShell native-command argument forwarding. The v0.3 helper used `$Args`, which collides with PowerShell's automatic `$args` variable and caused checks such as `git --version`, `rustup --version`, `rustc --version`, and `cargo --version` to execute without their arguments.
- Renamed `$host` to `$rustHostInfo`; `$Host` is a built-in read-only PowerShell variable and PowerShell variable names are case-insensitive.
- No product/task-graph changes. v0.3.1 supersedes v0.3 for Windows bootstrap use.

## Bootstrap v0.3
- Windows 11 x64 restored as primary development environment.
- Preserves v0.2 ProductIntent, execution-safety, evidence-authority, MCP and resumable-autonomy improvements.
- Native MSVC + PowerShell-first bootstrap/release path.
- Windows hook/installer/path/process requirements made release-critical.
- macOS/Linux retained as required cross-platform core targets.

## Bootstrap v0.2
- macOS-first development workflow experiment.
- Added ProjectIntent, explicit execution trust, external authoritative evidence, MCP bridge, improved plugin architecture and persistent resumption.

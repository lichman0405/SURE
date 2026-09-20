# Claude Code integration

Priority: first-class, target Tier 2 where current hooks permit.

The tier a `sure check` reports for a project is the one its **recorded events**
earn, not this line and not the adapter's own claim: Tier 1 for a project whose
store holds a session's events, and Tier 0 for a project with none — and never
Tier 2 from events alone, because events prove what happened rather than that
SURE could have stopped it.

Package root: `integrations/claude-code/`

Current bootstrap structure follows Claude Code plugin conventions:

```text
.claude-plugin/plugin.json
commands/
hooks/hooks.json
scripts/
```

Goals:
- user keeps normal Claude Code workflow;
- session lifecycle is observed where exposed;
- tool/command results are ingested as evidence where exposed;
- pre-tool protection may block/warn dangerous actions;
- `/sure:check`-style command path invokes the local core;
- selected SURE repair contract is handed back to Claude;
- SURE re-checks after repair.

The hook launcher is intentionally thin. On Windows, prefer PowerShell/direct `sure.exe` invocation using an explicit installed path or reliable per-user PATH. Do not require Bash. Secondary Unix launchers may use normal PATH resolution. Installation must fail safely if the binary is unavailable.

Before release, implementation must validate the exact hook event names/input/output semantics against the installed/current Claude Code version and maintain synthetic fixtures for those payloads.

# Claude Code integration

Priority: first-class, target Tier 2 where current hooks permit.

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

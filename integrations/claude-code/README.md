# SURE Claude Code Plugin

Priority: first-class; target Tier 2 where current Claude Code hooks permit.

The bootstrap package is **Windows-first**. Its current hook template uses PowerShell, which Claude Code officially supports for Windows command hooks. Release tasks must validate the installed/current hook schema and generate/package appropriate launchers for secondary macOS/Linux support.

The hook is intentionally thin: it forwards stdin JSON to the local SURE core. Core checking/protection logic does not live in the script.

The integration must fail safely if `sure.exe` is not installed and must never invent evidence for a hook that did not run.

# Bootstrap evolution

This Windows-first v0.3 supersedes both the earlier Windows v0.1 bootstrap and the macOS-first v0.2 package.

Do **not** revert to the older v0.1 design. v0.3 keeps the important architectural additions introduced in v0.2:

- ProjectIntent provenance/trust model;
- explicit execution modes (`inspect_only`, `host_confirmed`, container/sandbox where available);
- dependency install/network access separated from generic execution permission;
- project configuration cannot grant itself higher authority;
- authoritative evidence/history stored outside the checked working tree;
- stdio MCP bridge shared across harnesses;
- Cursor Plugin/hooks instead of prematurely building a heavy VS Code extension;
- persistent task state + HANDOFF + `/resume-sure`;
- expanded adversarial/product evaluation plan;
- self-dogfood before release.

v0.3 changes the primary development host back to Windows 11 x64 and makes PowerShell/native MSVC behavior first-class.

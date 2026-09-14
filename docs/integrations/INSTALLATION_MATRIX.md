# Integration installation matrix

## Claude Code
Development: local plugin install/symlink using Claude Code's supported plugin workflow.
Release: documented plugin package/marketplace route as appropriate.

## Cursor
Development: local Cursor Plugin under `~/.cursor/plugins/local/sure` (symlink preferred for iteration when allowed).
Release: Cursor Marketplace submission is external/account-review work and is not required for core v0.1 code completion.

## Codex
Development: local Agent Plugin/skill install according to the current Codex/OpenAI plugin/skill surface.
Release: package and instructions; marketplace/hosted publication is external if applicable.

## CLI-only fallback
Every user can run `sure check` without installing a harness plugin.

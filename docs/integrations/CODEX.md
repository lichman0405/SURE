# Codex integration

Priority: Tier 1 minimum for v0.1; deeper event/protection support where current public interfaces permit.

Baseline package:

`integrations/agent-plugin/`

This supplies a portable Agent Plugin/skill teaching the harness to invoke the local SURE engine and preserve SURE truth semantics.

Required v0.1 behavior:
- project check can be invoked from a Codex workflow;
- repair contract can be handed back to Codex;
- available evidence is ingested if supported;
- missing lifecycle visibility is explicitly reflected in the capability tier.

Do not block core v0.1 on achieving Claude-level hook parity if Codex exposes a different integration surface.

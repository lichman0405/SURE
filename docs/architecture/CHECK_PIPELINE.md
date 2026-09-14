# Check pipeline

1. **Discover** project/workspace, stacks, components, declared commands, config references and support level. The filesystem half of this step — which files SURE will look at, and which it will not — is `docs/architecture/PROJECT_DISCOVERY.md`; the reading half is `docs/architecture/ECOSYSTEM_DISCOVERY.md`; turning what was read into the places a verdict can be about is `docs/architecture/COMPONENT_GRAPH.md`.
2. **Resolve intent** from explicit/observed/documented sources without converting inference into requirement.
3. **Fingerprint** the current relevant project state. Which files that covers, what it deliberately leaves out, and why the digest is over file contents rather than over Git's diff, is `docs/architecture/FINGERPRINTING.md`.
4. **Plan** static/dynamic checks and execution trust requirements.
5. **Static deterministic checks** that do not execute project code where possible.
6. **Approved dynamic checks**: build/test/start/probes under the selected execution mode.
7. **Completeness analysis**: mocks/stubs/fake success/dead UI/broken glue/missing migrations/etc.
8. **Grounded model assessment** if configured.
9. **Claim checking** against observed evidence/current state.
10. **Aggregate** into findings + checked/not-checked scope + verdict.
11. **Repair contract** for selected finding(s).
12. **Re-check** affected and regression checks after repair.

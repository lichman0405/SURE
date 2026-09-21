# ADR 0002 — Local-first privacy

Status: Accepted

## Decision

Source code and evidence remain local by default. External model analysis is
explicit and disclosed.

- Default privacy mode is `local_first`; `fully_local` is available.
- `analysis.provider` defaults to `disabled`, and no-model mode must remain
  useful rather than degraded.
- Standard recording keeps event/tool type, command metadata and outcome,
  timestamps, file/Git activity, build/test/start activity and selected minimal
  claims. It does not require durable storage of full prompts, responses or
  terminal output.
- Full recording is opt-in, clearly distinguishable and deletable.
- Authority for privacy settings lives outside the project. A project file may
  request a behaviour; it cannot grant itself one
  (`execution::ConsentGrantor::can_grant`).
- Authoritative evidence and history live in user-level application data outside
  the working tree, so project code cannot fabricate a passing history.

## Frozen semantics

See `docs/architecture/FROZEN_SEMANTICS.md` and `docs/security/PRIVACY.md`.

## Revisit when

Concrete product evidence shows this decision materially blocks the core user job.

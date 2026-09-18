# Privacy

## Default: local-first

Source code and development evidence remain local by default.

No silent telemetry is required for v0.1.

## Standard recording

Capture where supported:
- event/tool type;
- command metadata/outcome;
- timestamps;
- file/Git activity;
- build/test/start activity;
- selected minimal AI claims;
- minimal structured project intent when allowed.

Do not require durable storage of full prompt/response/terminal content.

## Full recording

Explicit opt-in may store:
- full prompts;
- full agent responses;
- full terminal output;
- detailed tool payloads/edit history.

## External analysis

Three modes, named in `sure.yaml` as `privacy.mode`. What each one permits,
forbids, and what to write instead of the one this release does not implement, is
frozen in `docs/architecture/PRIVACY_AND_MODEL_STRATEGY.md`; this section is the
requirement those were built against.

- Local-first (`local_first`, the default) — external analysis only when
  configured, with bounded context.
- Fully local (`fully_local`) — no source code sent to external model services.
  A settings file naming this mode and an external provider is refused.
- Cloud-enhanced (`cloud_enhanced`) — explicitly enabled external analysis/sync
  features in future. **Not implemented:** the setting is refused rather than
  accepted, so no project can claim an arrangement SURE does not provide.

The mode in effect is the stricter of the two settings files — the project's
`sure.yaml` and the user's own file outside the project — so a project cannot
loosen what the user set.

**The report states which mode was in effect and whether a model was
consulted**, on every run that reads settings, including a run that stopped. The
no-model case is stated rather than omitted: silence would read both as "nothing
left this machine" and as "SURE did not look". In this release no check asks for
model-backed analysis, so the answer is always that no model was consulted — and
a run that cannot support even that says so instead.

## Deletion

Users must be able to inspect and delete local SURE history/recordings.

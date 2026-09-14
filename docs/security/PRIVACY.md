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

Modes:
- Local-first — external analysis only when configured, with bounded context.
- Fully local — no source code sent to external model services.
- Cloud-enhanced — explicitly enabled external analysis/sync features in future.

The report states when external analysis was used.

## Deletion

Users must be able to inspect and delete local SURE history/recordings.

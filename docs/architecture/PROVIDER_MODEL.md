# Model / analysis provider model

SURE v0.1 does not operate a hosted model service.

Provider options may include:

- `disabled` — deterministic/no-model mode;
- local command/model provider;
- user's Claude CLI/configuration;
- user's OpenAI-compatible endpoint.

Every model output is a `model_assessment` and must cite concrete anchors for material findings.

Provider failure:

- does not erase deterministic results;
- becomes visible `error/unknown` for the model-assisted portion;
- never converts into a pass.

The report states whether external model analysis was used.

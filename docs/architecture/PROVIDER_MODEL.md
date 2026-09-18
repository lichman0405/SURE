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

The report states whether external model analysis was used. It does that now, in
`details.model_use.state` and in the prose of a check report, with one state for
each way a run can end: no provider configured, a provider nothing asked,
a provider SURE could not build, a model consulted, and a run that cannot
confirm. `docs/architecture/PRIVACY_AND_MODEL_STRATEGY.md` is the semantics, and
it records the honest limit: **no check in this build asks for model-backed
analysis**, so the fourth state is unreachable and nothing is ever sent.

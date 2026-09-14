# Project intent / requirements

This is a critical truth boundary.

SURE cannot know what the user originally wanted merely by looking at code.

## Intent sources

Each requirement/goal records its source:

### explicit_user_goal
User supplied a goal/spec directly to SURE.

### observed_user_request
A supported harness exposed the user request and the selected privacy mode permits SURE to retain/normalize it.

### project_spec
README/spec/task file found in the project. This is documentation, not necessarily the user's current intent.

### agent_claim
Agent says a feature is complete. Useful for claim checking, not proof of requirement.

### inferred
SURE inferred likely purpose. Never treated as a user requirement.

## After-the-fact mode

Without explicit intent, SURE may say:

- the project starts;
- a button is dead;
- a payment path is mocked;
- README claim is false;
- a migration is missing.

It may **not** say:

> "Everything the user requested is complete."

## Full-session mode

Standard recording should avoid raw prompt retention by default.

Possible strategy:
- transiently observe the task where supported;
- retain a minimal structured goal/acceptance summary only when permitted;
- full raw prompt retention remains full-recording/opt-in.

If goal extraction requires an external model, disclose that model use and respect fully-local mode.

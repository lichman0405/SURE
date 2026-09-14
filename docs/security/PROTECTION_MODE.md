# Protection mode

User-facing modes:

## Standard — recommended
Warn/block only clearly dangerous operations where the integration has Tier 2 pre-action control.

## Strict
Also asks before additional sensitive changes such as migrations, CI/CD config, secret/config areas and broad filesystem modifications.

## Custom
Advanced rules.

Default UX explains consequence, not policy jargon.

Example:

> AI is about to delete 47 files. This may be a valid refactor, but it can also remove working code.
>
> [Allow once] [Do not allow]

Every integration must document whether its hook failure behavior is fail-open or fail-closed for the relevant event.

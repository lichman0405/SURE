# Protection mode

User-facing modes:

## Standard — recommended
Warn/block only clearly dangerous operations where the integration has Tier 2 pre-action control.

## Strict
Also asks before additional sensitive changes such as migrations, CI/CD config, secret/config areas and broad filesystem modifications.

## Custom
Advanced rules.

**Not implemented in this release.** `custom` needs a rule editor and a rule
format, neither of which exists, and a settings file naming it is **refused**
rather than accepted — see the `protection` table in
[CONFIG_REFERENCE.md](../architecture/CONFIG_REFERENCE.md#protection), which is
the same shape as `privacy.cloud_enhanced`. A hook that meets the value anyway —
or any settings file it cannot read at all — answers under `strict`, the firmest
mode this release implements, and never under the default: a user who cannot be
asked must not end up with less protection than they set.

Default UX explains consequence, not policy jargon.

Example:

> AI is about to delete 47 files. This may be a valid refactor, but it can also remove working code.
>
> [Allow once] [Do not allow]

## What the mode changes in this release

The mode is read from the user's settings file and the project's, and the
**stricter of the two** runs: a project may ask for more protection than the
user set and may not ask for less
([CONFIG_AUTHORITY.md](../architecture/CONFIG_AUTHORITY.md#restrictions-the-stricter-of-the-two-wins)).

`strict` differs from `standard` on the categories named above and on nothing
else. A **change** — a write or a delete — is held when it lands in a migration,
in CI/CD configuration, in a secret, or in SURE's own configuration area; so is
a change that names a whole location rather than a file (`.`, `..`, a drive
root, a wildcard, or a request that names no path at all), because that is the
shape of "delete 47 files". A **read** is held only where the read is the
sensitive act — a secret file — which is what "secret/config areas" means for an
operation that reads. Everything else is answered exactly as `standard` answers
it, so a read of ordinary source is not a mode difference.

Every answer carries a sentence in the user's terms rather than the policy's: a
held read of `.env` says the file is where credentials live, and a held change
across a whole location says the change may be a valid refactor and may also
remove working code. An allowed request carries the sentence it was allowed
with, so a decision with no sentence means no request was put to the rule at all
— a lifecycle event, not a verdict on a tool.

**The answer is advisory in this release.** Both integrations are capability
tier 1 (Observed) and their manifests do not confirm that the harness interprets
or enforces the response, so a decision states what SURE *would* do and not what
the harness did. Nothing in this build can establish the second sentence.

Every integration must document whether its hook failure behavior is fail-open or fail-closed for the relevant event.

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

## The one-time override, and what it reaches

The example above shows `[Allow once]`. In this release the way a user gives
that answer is a command:

```
sure hook allow-once --tool NAME (--command WORDS | --path PATH)
                     [--project DIR] [--minutes N]
```

It records a grant in SURE's store for **one** request: the tool name and the
command line or the path, exactly as the harness spells them. The first request
that matches both exactly **spends** it, and if SURE names that request as one of
the three acts below, SURE would let it through. The next request with the same
words is held again. The window is thirty minutes unless the user says otherwise,
never more than a day, and the spend is one store transaction — two hooks racing
on one allowance cannot both spend it.

An allowance reaches the three acts this document calls dangerous and nothing
else:

- a change that names a whole location rather than a file;
- a push that would replace commits that were already published;
- a read of a file where credentials or keys live.

A request held for any other reason cannot be let through by one — a mode that
does not permit it, an execution setting the user has not changed, a migration,
CI/CD configuration, SURE's own configuration area — and neither can a request
whose command line has more than one reading, or one SURE cannot call destructive
from its own rule table. That is the fail-closed direction on purpose: an
allowance answers a question a hook cannot ask, and it is not a way to run under
settings the user did not change.

Nothing here is a claim that a harness honours the answer. Both integrations are
tier 1 (Observed) and a spent allowance produces a sentence saying SURE **would**
let that one request through; whether anything then runs is not something this
build can confirm — see the paragraph above.

`sure hook allow-once` is a command a **person** runs. Nothing a checked project
can write and nothing an agent can say records an allowance: the grant is a row
in SURE's own store, written on a command line the user typed. The subject is not
checked against the rule table at write time, because there is no request to read
then — the danger is named when a request arrives, and a grant no request ever
matches is spent by nothing and expires.

Every integration must document whether its hook failure behavior is fail-open or fail-closed for the relevant event.

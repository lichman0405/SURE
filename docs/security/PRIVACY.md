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

The opt-in is `privacy.full_recording`, off by default. **Only the user's own
settings file can turn it on** — a project naming it is a request its own file
cannot grant — and the report prints the answer on every run that reaches one,
including a run that stopped before a verdict, so a session that kept a full
transcript says so rather than leaving it to be discovered afterwards.

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
consulted**, on every run that gets past reading its settings, including a run
that stopped before a verdict. The no-model case is stated rather than omitted:
silence would read both as "nothing left this machine" and as "SURE did not
look". In this release no check asks for model-backed analysis, so the answer is
always that no model was consulted — and a run that cannot support even that says
so instead. A run that **cannot accept** a settings file is the one case with no
mode to state: it stops, names the setting it could not honour, and exits 5
without a privacy section, because the mode in effect is not something such a run
ever established.

## Deletion

Users must be able to inspect and delete local SURE history/recordings.

Both are commands as of P13-T003, and neither answers about a project:

- `sure history` lists the sessions this machine has recorded — which project
  and harness each came from, and how long it is kept —
  and `sure history show <SURE_SESSION_ID>` prints one session with the events
  in it, the record each event wrote, and the decision SURE reached about it
  where the event asked for one: the action, the danger it named, the tool, the
  reason the user was given, and the allowance it spent if it spent one. The
  request's own words are not repeated there — they are already in the event the
  decision hangs from, redacted once — so the decision is a row about the
  decision rather than a second copy of what was asked.
- `sure history delete --session <SURE_SESSION_ID>`, `--project <ROOT>` or
  `--all` removes what the scope names: the `sessions` row, its
  `session_events`, the `records` those events own, the decision rows recorded
  for them, and any full recording written for one of them. It reports each
  count separately, because "the raw transcript is gone too" is a different
  claim from "the session row is gone". All of it goes in one transaction: a
  delete that stops partway removes nothing.

Three properties are part of the promise, not of the implementation:

- **Nothing is deleted on a schedule.** Every row carries a date it is kept
  until, and nothing in this release acts on one. The listing says so next to
  the date, because a date printed alone reads as a retention policy that is
  being enforced. Deleting is something the user does.
- **Nothing is deleted without a scope on the command line.** Exactly one of
  `--all`, `--session` and `--project` is required; a command line naming none
  or two is a usage error (status 2) rather than a default. There is no prompt
  and no confirmation, so this is what keeps a delete from happening by
  omission — and it is why the command is usable from a script at all.
  A delete whose scope matched nothing says it removed nothing and exits 0.
- **A project cannot extend how long its own records are kept.** The duration is
  a setting, and it resolves like every other restriction: a project file may
  shorten it and may not lengthen it. A project asking for longer leaves a
  refused request on the record and the shorter period is what is written. The
  setting is `privacy.full_recording_retention_days` in
  [CONFIG_REFERENCE.md](../architecture/CONFIG_REFERENCE.md#privacy); the rule
  is [CONFIG_AUTHORITY.md](../architecture/CONFIG_AUTHORITY.md#restrictions-the-stricter-of-the-two-wins).

What a delete does **not** do: it does not touch the project's files or the
project's configuration. **It does not always spare another project's records
either** — the scope is what decides that, and `--all` is "every session in this
store" in the command's own words, so it reaches one project's sessions and every
other project's alike. A user asking SURE to forget something must not find out
later that it forgot more than it said, which is why the breadth is the scope's
word to give and never a default.

**A delete removes sessions, and a session is not the whole store.** What it
reaches is what the sessions own: their `session_events`, the `records` those
events point at, and the recordings and decisions joined to them by event id.
Rows no session event points at — a recorded goal, a standard projection, an
approval — are not session records, so not even `--all` reaches them, and nothing
a user can run removes them in this release. That is a limit on the promise above
rather than a defect in the delete, and it is stated because a user reading
"every session in this store" is entitled to know that the store and its sessions
are not the same thing.

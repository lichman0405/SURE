# Diagnostics

SURE records two different things about a run: what it found out about the
project, and what SURE itself did. This document is about the second one.

The distinction is load-bearing. `docs/architecture/EVIDENCE_MODEL.md` puts
SURE's own activity at the bottom of the truth order, and a diagnostic is never
evidence about the project. "The check was planned" is not "the check passed".
Nothing in `sure_core::diagnostics` can become a `CheckResult`, and the types are
separate so that it cannot happen by accident.

Diagnostics are also not the report. The report is user-facing prose written to
the rules in `docs/product/UX_AND_LANGUAGE.md`. Diagnostic lines are for
`sure doctor`, for a bug report and for SURE's own maintainers: terse,
lowercase, machine-readable. Nothing here belongs in a verdict.

## A line

```text
2026-09-14T09:10:56.827Z warn  run_01j2m8q5… session=ses_… check=chk_…  config was not read  path="sure.yaml" reason="it is a directory"
```

| Part | Fixed? | Notes |
| --- | --- | --- |
| timestamp | fixed | RFC 3339, UTC, milliseconds, always 24 characters |
| level | fixed | `error`, `warn`, `info`, `debug`, `trace` |
| correlation | fixed | `run`, plus `session=` and `check=` when they are known |
| message | fixed | a `&'static str` |
| fields | — | `key="value"`, `key=42`, `key=true`, `key=<not recorded>` |

## Correlation

| Field | Always present | Identifies |
| --- | --- | --- |
| `run` | yes | one execution of SURE |
| `session` | no | the harness session SURE is observing |
| `check` | no | the check currently being evaluated |

A run exists whether or not a harness is present, which is why it is never
optional: `sure check` started by hand produces lines with a run, no session,
and a check while checks run. A session is only known when SURE was started by
an integration, and it is **omitted** rather than invented when it is not known
(`docs/architecture/EVENT_PROTOCOL.md`: "Missing data is not invented"). A line
that said `session=none` would invite the reading that SURE looked and found
nothing, which is a different statement from not having been told.

`session` and `check` are not alternatives. A check runs inside a session when
there is one, and the two answer different questions: what the agent was doing,
and what SURE is doing now.

`Correlation::for_check` derives a check-scoped correlation from the run's, so
every line emitted inside a check carries the identities that lead back to the
run that produced it.

## Why the message is a literal

`Diagnostic`'s message is `&'static str`, which is the same rule as
`sure_domain::status::NO_TRUSTED_INTENT_LIMITATION` being a `const`: a sentence
SURE says is fixed, so no call site can paraphrase it into something weaker and
no project-controlled text can be interpolated into it. Everything variable
goes in a field.

Field *keys* are `&'static str` for the same reason. A key built from
project-controlled text would let a project choose what a log line appears to
say.

## Secrets

`docs/security/SECRET_REDACTION.md` requires that redaction happen before a
durable write, and that a secret never have to be handled in order to be
reported. Three properties make the second one true:

1. `Field::redacted("api_key")` records that a credential was present and
   **takes no value**. The code path that knows about a secret does not have to
   pass one along to report accurately.
2. `Field::text("api_key", …)` discards the value without inspecting it,
   because the decision is made from the name. Running the pattern list over a
   value whose key already says it is a credential would suggest the patterns
   are what protects it.
3. Every other value passes through `sure_core::redact` before it is stored, in
   `Field::text`, which is the only route into a record.

Two markers, deliberately distinct:

| Marker | Means |
| --- | --- |
| `***` | this value looked like a secret and was replaced |
| `<not recorded>` | this field is never recorded |

A reader of a log can therefore tell a redaction they should investigate from an
omission by design.

None of this claims to detect every secret. `docs/security/SECRET_REDACTION.md`
is explicit that detection is imperfect, and the structural rules above matter
more than the patterns.

## One line, always

A value is written inside a quoted span with backslashes, quotes and control
characters escaped, so it cannot close its own quote, open a new `key=`, or add
a line that looks like it came from SURE. Backslashes and quotes are escaped
first and control characters second, which is what keeps a real newline (`\n`)
distinguishable from a literal two-character `\n` (`\\n`); escaping in the other
order would render both the same.

The upshot is that a log file can be read by splitting on `\n`.

## Where lines go

To a `Write` chosen by the caller: stderr, or a local log file. There is
deliberately **no constructor that writes to stdout**. `sure hook ingest` emits
the harness response contract on stdout, and debug noise there would corrupt it
(`docs/architecture/EVENT_PROTOCOL.md`). Making stdout unreachable by
construction is worth more than a rule in a comment.

A write that fails returns its error rather than being swallowed, so a caller
that ignores one is visibly ignoring it. A diagnostic that could not be written
is not a reason to abandon a check.

## Levels

`Level::from_verbosity` maps a verbosity count to a threshold. The CLI does not
expose the count yet — nothing emits diagnostics yet, see "Known gaps" — so this
is the mapping that command-line parsing will use, not a flag that works today.

| Verbosity | Threshold | Records |
| --- | --- | --- |
| none | `warn` | `error`, `warn` |
| `-v` | `info` | and `info` |
| `-vv` | `debug` | and `debug` |
| `-vvv` or more | `trace` | everything |

The default is `warn` rather than `info`: a check that prints a line per step
trains people to ignore the output, and the lines that matter are the ones that
are not routine.

## Not `tracing`

`docs/architecture/RUST_DESIGN.md` lists `tracing` as a likely dependency for
structured local diagnostics, and this module is not it. The reasoning, and the
condition that would change the answer, are in
`docs/adr/0012-diagnostics-are-records-not-a-logging-framework.md`.

## Enforced by

| Rule | Where |
| --- | --- |
| Correlation fields | `crates/sure-core/src/diagnostics/correlation.rs` |
| A value cannot restructure a line | `crates/sure-core/src/diagnostics/field.rs` |
| Redaction before storage | `crates/sure-core/src/redact.rs` |
| UTC timestamps | `crates/sure-core/src/diagnostics/time.rs` |
| No stdout sink | `crates/sure-core/src/diagnostics/mod.rs` (there is no such constructor) |

## Known gaps

Recorded so they are not forgotten, and resolved by the tasks named.

1. **No JSON encoding yet.** `Diagnostic` is a plain text line. The storage task
   (P1-T005) decides the durable log format, and that decision should be made
   once rather than guessed at here.
2. **Captured process output is not yet redacted.** `SECRET_REDACTION.md` asks
   for stdout/stderr to be redacted before a full recording is written. Nothing
   captures process output yet; the redaction it will need is in
   `sure_core::redact`.
3. **A write failure is not yet surfaced to the user.** The recorder returns the
   error, and nothing consumes it, because nothing emits diagnostics yet.

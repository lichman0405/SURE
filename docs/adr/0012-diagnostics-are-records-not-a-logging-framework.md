# ADR 0012: Diagnostics are records, not a logging framework

Status: accepted (P1-T006).

## Context

`docs/architecture/RUST_DESIGN.md` lists "tracing for structured local
diagnostics" among the likely dependencies. P1-T006 asked for structured
tracing and a redaction foundation: correlation fields for run, session and
check, and a guarantee that no raw secret value is *required* in a log.

`tracing` is the obvious reading of that. It brings spans that carry correlation
fields, levels, and a subscriber ecosystem, and it will be the natural choice
once there is asynchronous work whose spans have to survive an `.await`.

Two things argued against adopting it now.

**A redaction guarantee has to be enforced at the record, not the transport.**
`tracing`'s idiomatic form is `info!(?endpoint, "provider rejected")`, and any
call site can log whatever it likes; the framework has no opinion. A redacting
`Layer` filters what a subscriber writes, but the secret still existed as a
field, and the property "the code path that knows a secret never had to hand
one over to report accurately" is not expressible.

**There is no async code yet.** Spans earn their keep by propagating across
await points and across task boundaries. SURE has no daemon
(`docs/adr/0003-no-persistent-daemon-v01.md`) and no `tokio` yet, so the
value being paid for is not yet being collected, while the subscriber registry
— process-global mutable state — would be adopted immediately.
`RUST_DESIGN.md`'s own dependency strategy says to "choose current compatible
versions during implementation rather than preloading the bootstrap with
unnecessary dependencies".

## Decision

1. `sure_core::diagnostics` defines the **record**: `Level`, `Correlation`
   (run, session, check), `Timestamp`, `Field`, `Diagnostic`, and a `Recorder`
   that writes one line at a time to a caller-supplied sink.
2. A message is a `&'static str`, not a formatted `String`. Variable content
   goes in a field. This is the same rule as
   `sure_domain::status::NO_TRUSTED_INTENT_LIMITATION`: a sentence SURE says is
   a constant, so no call site can paraphrase it into something weaker and no
   project-controlled text can be interpolated into it.
3. Field keys are `&'static str` for the same reason. A key built from
   project-controlled text would let a project choose what a line appears to
   say.
4. `Field::text` redacts on construction and is the only route into a record.
   A key that names a credential causes the value to be discarded **without
   being inspected**; `Field::redacted` records that a credential was present
   and takes no value at all. This is what makes a secret unnecessary rather
   than merely discouraged.
5. Values are quoted and escaped so a value cannot close its own quote, open a
   new key, or add a line. A log file can be read by splitting on `\n`.
6. There is **no** constructor that writes to stdout. `sure hook ingest` emits
   the harness response contract there, and making the mistake unreachable beats
   documenting it.
7. `RunId` is added to the frozen identifier vocabulary with the prefix `run`,
   because a run is a durable entity — it is what a stored verdict, a
   diagnostic line and a history entry are correlated by — and it is not a
   `SessionId`, which is a *harness* session SURE observed.
8. `redact` moves from `sure_core::config` to the crate root. It was never a
   config concern; config was its first caller.
9. Diagnostics are not evidence. Nothing in `diagnostics` can become a
   `CheckResult`, and the types are separate so it cannot happen by accident.

## Alternatives rejected

**Adopt `tracing` and `tracing-subscriber` now, with a redacting layer.** The
correlation model would be better expressed as spans, and this is probably where
the product ends up. Rejected for now because it buys span propagation that
nothing uses yet, at the cost of a process-global subscriber and a redaction
story that lives in a layer rather than in the type.

**Adopt `tracing` for the plumbing and keep these types as the payload.** Two
frameworks for one job. When `tracing` arrives, it should replace `Recorder`,
not sit beside it.

**A logging facade with no implementation, to be filled in later.** A trait with
one implementation and no second consumer is a guess about the second consumer.

**Make the message a `String` and rely on caller discipline.** This is the
failure mode the ADR exists to prevent: nothing would stop a call site
interpolating a credential into a message that is then written verbatim.

**Reuse `sure-domain`'s `variants!` machinery for `Level`.** Levels are SURE's
own vocabulary, not part of the wire contract. Putting them in the domain crate
would make a change to logging a change to the frozen semantics.

## Consequences

- The redaction property is testable end to end: a credential handed to a real
  recorder, into a real buffer, does not appear.
- `Correlation` is a plain value, so a check runner threads it rather than
  relying on an ambient span.
- Adding `tracing` later means replacing `Recorder` and keeping `Correlation`,
  `Field` and `Diagnostic` as the payload. The record model is the part that is
  meant to outlive the decision.
- There is no JSON encoding yet. The durable log format is chosen once, by the
  storage task (P1-T005), rather than guessed at here.
- `IdKind` gained a variant, so `crates/sure-domain/tests/wire_contract.rs`
  stopped compiling until `run` was named. That is the intended behaviour of
  that test.

## Frozen semantics

- A diagnostic is never evidence about the project.
- A run is not a session. A run is one execution of SURE; a session is observed
  harness activity. Either can exist without the other.
- An unknown session or check is omitted, never rendered as a placeholder, so
  "SURE was not told" is never presented as "SURE looked and found nothing".
- `***` means a value looked like a secret and was replaced; `<not recorded>`
  means the field is never recorded. The two are not interchangeable.

## Revisit when

Asynchronous orchestration lands and spans have to propagate across `.await`
points or across tasks. At that point the `Recorder` is the thing to replace,
and `Correlation`, `Field` and `Diagnostic` are the things to carry over. Also
revisit if the durable log format turns out to need a machine-readable encoding
that this record cannot express.

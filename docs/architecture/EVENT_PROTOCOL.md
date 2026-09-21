# Harness event protocol

All harness adapters normalize events to a versioned local schema.

The Rust form of this document is `sure_protocol::event::EventEnvelope`, and
`schemas/event.schema.json` is the machine-readable one. How those two relate to
the other documents SURE writes, and what happens when a version does not match,
is `docs/architecture/PROTOCOL.md`.

Minimum envelope:

```json
{
  "schema_version": 1,
  "source": "claude-code",
  "capability_tier": 1,
  "session_id": "...",
  "event_type": "tool.completed",
  "timestamp": "...",
  "project_root": "...",
  "payload": {}
}
```

Adapters may omit fields not exposed by their harness. Missing data is not
invented.

`capability_tier` is a number: `0` snapshot, `1` observed, `2` protected. Absent
means the adapter did not say, which is **not** the same as `0` — a reader must
not collapse the two, because one is an honest "SURE cannot see the session" and
the other is "nobody told SURE".

`session_id` is the harness's own identifier for its session, not a SURE
`SessionId`. SURE cannot validate it and does not try to.

This document is closed: a field it does not define is refused rather than
ignored, so an adapter that sends something SURE does not understand gets an
error instead of a silent gap in what SURE claims it can see.

## Important event families

- session start/end/resume;
- user task/goal where privacy mode permits;
- agent completion response/claim candidate;
- shell/tool requested;
- shell/tool completed/failed;
- file read/edit/delete where exposed;
- Git activity where exposed;
- pre-action protection decision.

## StdIO contract

Hook launchers receive harness JSON on stdin and pass normalized/annotated input to `sure hook ingest`.

Stdout must conform to the harness hook response contract and must not contain debug noise.

Debug logs go to stderr/local SURE logs.

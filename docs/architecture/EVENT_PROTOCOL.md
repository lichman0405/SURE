# Harness event protocol

All harness adapters normalize events to a versioned local schema.

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

Adapters may omit fields not exposed by their harness. Missing data is not invented.

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

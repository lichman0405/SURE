# Long-running Claude Code session recovery

The project is deliberately designed to outlive one model context/session.

Durable truth:
- Git history;
- `tasks/tasks.json`;
- `progress/state.json`;
- `progress/HANDOFF.md`;
- test output/evidence files that tasks choose to persist.

Before stopping/compaction:
1. finish or explicitly leave current task in-progress;
2. update handoff with exact command/test failures;
3. commit accepted coherent work;
4. record Git status;
5. never mark incomplete work accepted.

Resume with:

```text
/resume-sure
```

The new session must verify state against Git/files instead of trusting an old narrative summary.

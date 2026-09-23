# Autonomous task graph

Phases and tasks: **not stated here**, for the reason `tasks/SUMMARY.md` gives. This file said
"17 phases / 168 tasks" while the graph held 18 phases and 204 tasks, and then 19 and 216: the
two numbers had been wrong for two phases before anyone read them, because nothing in the tree
reads them. The counts are printed from `tasks/phases.json` and `tasks/tasks.json` by
`node scripts/validate-bootstrap.mjs` and `node scripts/taskctl.mjs validate` on every gate run.

Use `node scripts/taskctl.mjs`.

Task status is persisted in `progress/state.json`; Git commits/tests are stronger evidence than narrative progress.

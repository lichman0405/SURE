# Task graph summary

- Product: **SURE — Software Understanding & Reality Evaluation**
- Primary local development: **Windows 11 x64 / native MSVC**
- Final core CI: **Windows + macOS + Linux**
- Phases and tasks: **not stated here**. A count typed into this file drifts from the graph the
  file sits in, and nothing in the tree reads it, so the two counts are printed by gates rather
  than restated: `node scripts/validate-bootstrap.mjs` and `node scripts/taskctl.mjs validate`
  read `tasks/phases.json` and `tasks/tasks.json` and print what they hold on every run.
- Autonomous branch: `claude/v0.1-autonomous`
- Canonical repo: `https://github.com/lichman0405/SURE.git`

# v0.1 definition of done

SURE v0.1 is done only when a clean user can:

1. install/build the local CLI/core on native Windows 11 x64;
2. run `sure doctor` and get actionable environment information;
3. run `sure check` on a supported project without any harness plugin;
4. choose/understand whether project code will execute;
5. receive a plain-language report with checked / not checked scope;
6. see fake/incomplete implementation in required adversarial fixtures;
7. receive `cannot_confirm` rather than a fabricated pass when evidence is insufficient;
8. use the Claude Code plugin for observed/protected checking;
9. use the Cursor Plugin for the supported capability tier;
10. hand a finding back as a repair contract and re-check it;
11. delete local history/recordings;
12. run without an external model provider and still get useful deterministic results.

Release gates:

- zero false green on the mandatory adversarial corpus;
- no fabricated command/test execution evidence;
- critical `error/skipped/unknown` never aggregates to clean green;
- secret-redaction fixtures pass;
- native Windows local gates pass;
- Windows/macOS/Linux Rust-core CI passes;
- install/uninstall docs pass clean-machine simulation as far as credentials permit;
- final report records exact limitations.

# The gate set, and the harness that runs it

`scripts/gates.ps1` is the tracked copy of the harness that produces the
`exits: fmt=0 clippy=0 test=0 bootstrap=0 taskctl=0 nonwindows=0` line every
acceptance in this repository quotes. Readings taken since
`scripts/product-evals.mjs` became the seventh gate end that line with
`productevals=<exit>`, appended *after* those six so the quoted fields keep
their positions and an old reading still lines up with a new one field for
field; the readings already in `progress/` are not retro-edited. Before it was
tracked that harness lived
at `target\tmp\gates.ps1` — inside the git-ignored `/target/` — so no clone of
this repository could reproduce a single reading the record contains, and a
`cargo clean` would have taken the instrument with it. `target\tmp\measure-run.mjs`
is the same defect arriving: it was lost, and eight of its invocations are still
quoted as evidence.

This file is the place that says how to invoke the gate set and what
environment the readings were taken under. `docs/development/WINDOWS.md` points
here for the gate-specific part and remains the place the rest of the Windows
environment is written down.

## How to run it

```powershell
pwsh -NoProfile -File scripts/gates.ps1 -Label p16t001-worker
```

- **From anywhere.** The repository root is found from the script's own path,
  never from the working directory. Every gate command runs with the repository
  root as its working directory.
- **PowerShell 7 or later.** The record's readings were taken under PowerShell
  7.6.6. The runner refuses by name under Windows PowerShell 5.1 rather than
  taking a reading in an environment the record does not describe.
- **`-Label` is mandatory and becomes a filename.** `gates-<label>.txt` is the
  summary and `gates-<label>-<gate>.txt` is each command's own output, all in
  `target/tmp/`. The label must be letters, digits, dot, dash and underscore
  beginning with a letter or a digit; anything else is refused, because a label
  that reaches outside `target/tmp` is a reading written over something else.
- **`-PreflightOnly`** runs every precondition and the suppression census and
  stops before the seven commands. It writes `gates-<label>-preflight.txt` and
  never touches `gates-<label>.txt`, so a preflight cannot overwrite a reading.
  Use it to check an environment before spending a build on it.
- **`-AllowRefusedChildPolicy`** takes the reading anyway when the shell a test
  target starts cannot load a `.ps1`. See below: the log then says out loud that
  the test gate is expected red and that the reading is not comparable with one
  taken from a session that permits scripts.

### The target directory: a clone builds in the clone's own

**Do not point `CARGO_TARGET_DIR` at another checkout.** Cargo's uplift step
writes `target/debug/sure.exe`, `target/debug/deps/*` and the test binaries to
one place per target directory, so two trees sharing one target directory take
turns owning those names, and a run in one tree can be served an artifact built
from the other tree's sources. Measured on 2026-09-21 (`P15-T037`), a clone whose
gate run was pointed at the primary checkout's `target/`:

- produced no `target/debug/` of its own, and its own gate summary recorded the
  build as successful, so nothing in the reading itself says which tree's
  artifacts ran;
- `target/debug/sure.exe`, the shared artifact name, was rewritten inside the
  same window (mtime `10:14`);
- the clone's `cargo test` log grew to **150 MB** and its middle is **raw x86
  machine code**, which a single-writer build does not put into a test log; the
  log's first run, in the same arrangement, was 245 KB of text. Two builds
  sharing one `target/debug/deps/` is the mechanism that explains it, and it is
  named here as the explanation rather than measured — what is measured is the
  size, the bytes, and the fact that the shared name `target/debug/sure.exe` was
  rewritten in the same window;
- that reading is **withdrawn**: it is not quoted as evidence for anything, in
  this file or in the hand-back that raised it.

The rule is one line: **a clone's gate run uses the clone's own `target/`**,
which is what happens when `CARGO_TARGET_DIR` is simply not set — `cargo`'s
default is `<workspace root>/target`. Nothing in this runner sets it, and the
first run in a fresh clone is a cold build of the whole workspace, which is the
price of the isolation.

### Exit codes

Every failure is in the log by name; the exit code is the one line a script
reads.

| Code | Meaning |
| --- | --- |
| 0 | all seven gates exited 0, the census is clean, both counts were attributed |
| 1 | at least one gate exited non-zero |
| 2 | a precondition is missing (edition, tree, tools, gate table); no gate ran |
| 3 | the shell a test target starts cannot load a `.ps1`; no gate ran |
| 4 | a suppression token was found in the harness |
| 5 | the test log exists but no count can be attributed from it |

## The seven commands

Run in this order, each with its own log file:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --no-fail-fast
node scripts/product-evals.mjs
node scripts/validate-bootstrap.mjs
node scripts/taskctl.mjs validate
node scripts/check-non-windows.mjs
```

They are not repeated in this file as the definition. The definition is the
`$GateSet` table in `scripts/gates.ps1`, where each row carries both the
command line a reader would type and the argument array that actually runs, and
the preflight compares the two before any gate starts. A change that reaches one
and not the other refuses with `refused [gate-table]`, naming both lines, rather
than producing a reading under a command line nobody wrote down.

Three of the seven are cargo subcommands and four are scripts the tree already
carried — `scripts/validate-bootstrap.mjs`, `scripts/taskctl.mjs`,
`scripts/check-non-windows.mjs` and `scripts/product-evals.mjs`.
`scripts/check-non-windows.mjs` was the sixth gate until the seventh was added
and says in its own header why it is a gate and what it cannot reach.

### The seventh gate, and why its row is not free to move

`node scripts/product-evals.mjs` reads `docs/product/PRODUCT_EVALS.md` against
`target/tmp/release-gate.json` and exits 1, naming the command that produces the
reading, rather than printing a number it did not read. Its input is the `test`
gate's own artifact, and that is the whole reason its row is where it is: move it
anywhere before `test` and it would compare the document against a gate written
by an earlier run, or find none and refuse. Its row is the only one in the table
whose position carries a constraint, and the list above is the order the table
has, not a preference.

What it costs, measured by the supervisor on 2026-09-21:

- **77 ms**, `time node scripts/product-evals.mjs` → `real 0m0.077s`. Node and
  the checkout are the whole of the requirement: no network, no package install,
  no administrator rights, no shell beyond the one that starts `node`.
- its input is written in **0.86 s** by
  `crates/sure-core/tests/acceptance_report_runner` (14 passed, 0 failed), which
  the `test` gate runs as part of
  `cargo test --workspace --all-features --no-fail-fast`.

It was wired to nothing before this row existed, and that was not a neutral
omission. `P15-T026` measured the gate side of it: moving the repair-regression
line of `docs/product/PRODUCT_EVALS.md` from `**100%**` to `**99%**` leaves
`cargo test -p sure-core --test release_gate_runner` at **16 passed; 0 failed**.
The other half is what this row changes: `node scripts/product-evals.mjs` exits
1 with `WRONG` on the same edit. A false number in a shipped product document
that every automatic check passed is the defect this repository exists to catch.

## The environment the readings were taken under

This is the part `P15-T036` measured and this file exists partly to write down:
**the conditions the readings are taken under are not the machine's defaults,
and a reader in an ordinary PowerShell window is not reproducing them.**

Measured on this machine, 2026-09-21:

```text
Get-ExecutionPolicy -List
  MachinePolicy       Undefined
  UserPolicy          Undefined
  Process             Bypass
  CurrentUser         Undefined
  LocalMachine        RemoteSigned
PSExecutionPolicyPreference = Bypass
```

The `Bypass` is **injected by the tool harness that launches `pwsh`**, not set
by this repository, not set by any script in it, and not present in a PowerShell
window a person opens by hand. It is a `Process`-scoped value, which is where
PowerShell keeps `PSExecutionPolicyPreference`; every descendant inherits it and
it writes nothing to any scope and nothing to the registry.

It matters because six workspace test targets start Windows PowerShell with a
`.ps1`, and Windows PowerShell's own default when no scope sets anything is
`Restricted`. Measured the same day: a session that carries the process-scoped
`Bypass` gets a green `test` gate, and a session that does not gets **29
`UnauthorizedAccess` failures across those six targets**, every one of them the
shell refusing to load a file rather than the tree. `docs/development/WINDOWS.md`,
`## Which shell the tests are run from`, carries the table and the raw message.

### What the runner requires, and what it does when it does not have it

It does not infer this from a policy name. It **measures** it: it writes one
probe `.ps1` into the temp directory, starts `powershell.exe -NoProfile
-NonInteractive -File <probe>` as a child of its own process, and reads the exit
code. Exit 0 means the shell a test target starts can load a script; anything
else means it cannot, and the log then carries the child's own words, the
child's own `Get-ExecutionPolicy -List` rows, and this:

```text
policy: REFUSED -- C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -File <probe> exited 1 instead of 0
  <the host's own refusal, verbatim>
  Six workspace test targets start that shell with a .ps1, and Windows PowerShell
  answers Restricted when no scope sets one: 29 UnauthorizedAccess failures across
  those six targets, every one of them the shell refusing to load a file.
  What it requires: a PROCESS-scoped policy that permits scripts, carried by the
  session that starts cargo test, inherited by everything under it:
      pwsh -ExecutionPolicy Bypass -File scripts/gates.ps1 -Label <label>
  or, in the session you already have:
      Set-ExecutionPolicy -Scope Process Bypass
```

and then it **refuses to run the gates** and exits 3. Running anyway would
produce a red `test` gate that is not the diff, which is worse than no reading:
this repository has already paid for that once, in the six targets that report a
host refusal as though it were a launcher failure.

**The runner never sets an execution policy at any scope and writes no registry
key.** It recommends the process-scoped one and nothing else, because a
machine- or user-scoped change would make this refusal invisible on every
machine it was applied to, and a check that cannot report a refusal is the false
green this repository treats as more serious than a visible error.

The probe file is removed afterwards, and if it cannot be removed the log says
so: `target/tmp` and the temp directory are both walked by tests.

## The two counts, and which one to quote

`passed=` in the summary is the **RAW SUM over every `test result:` line**. It
is not the number of tests. On two logs in the record:

```text
$ node scripts/measure-tests.mjs target/tmp/gates-p15t035-accept-test.txt target/tmp/gates-p15-batch-accept-test.txt
target/tmp/gates-p15t035-accept-test.txt
  raw sum              2754   every `test result:` line, children included
  parent figure        2744   `test <name> ... ok` lines, one per test a parent ran
target/tmp/gates-p15-batch-accept-test.txt
  raw sum              2772   every `test result:` line, children included
  parent figure        2762   `test <name> ... ok` lines, one per test a parent ran
```

The difference is children: `crates/sure-core/tests/store_concurrency.rs` spawns
ten children that re-run a parent's test with `--exact`. A child prints its own
`test result:` line and no `test <name> ... ok` line, so the raw sum counts that
test twice. **Both figures were published wrongly in this record** — `passed=2754`
in `1052316` and in `b34ad94` as though it were a count of tests, when the
correct figure is 2744.

So the summary prints both, and says which is which:

```text
result-lines=88 passed=2772 failed=0 ignored=12 not-ok=0
counts: passed= above is the RAW SUM over every `test result:` line. It is NOT the number of tests.
counts: parent=2762 -- `test <name> ... ok` lines, one per test a parent ran (scripts/measure-tests.mjs exit 0)
counts: quote parent= as the number of tests; the difference is children that re-ran a parent's test.
```

The parent figure is **called, not reimplemented**: the runner runs
`scripts/measure-tests.mjs`, the tracked instrument `P15-T021` put in the tree,
and surfaces its exit code. When that instrument refuses — a truncated log, a
log whose output passed through a filter — the runner prints
`counts: parent=REFUSED` with the instrument's own reason, says that `passed=` is
not a count of tests either, and exits 5. A second implementation of a number
that decides an acceptance would be two instruments disagreeing later, not
redundancy now.

## What the runner refuses, and what each refusal was exercised against

| Refusal | When | Exercised |
| --- | --- | --- |
| `[no-script-path]` | the file's own path is empty, so it cannot say where it is | not exercised |
| `[bad-label]` | the label would leave `target/tmp` | not exercised |
| `[ps-edition]` | running under Windows PowerShell 5.1 | not exercised |
| `[not-a-checkout]` | the tree this file sits in is missing one of the files it needs | not exercised |
| `[tools-missing]` | `cargo`, `node` or `git` is not on `PATH` | not exercised |
| `[gate-table]` | the written command line and the argument array are not the same command | measured: the fmt row's `Command` changed to `cargo fmt --check` and the gate run refused with `refused [gate-table]: gate 'fmt': the command written down is not the command that would run / written down: cargo fmt --check / would run: cargo fmt --all -- --check`, exit 2 |
| `[child-policy]` | a child `powershell.exe` cannot load a `.ps1` | measured twice: exits 1 with `UnauthorizedAccess` from a shell carrying no `PSExecutionPolicyPreference`, exit 0 and a green run from one carrying `Bypass` |
| `counts: parent=REFUSED` | `scripts/measure-tests.mjs` refuses the log | the instrument's own refusal is measured in `measure-tests.mjs`'s header (two logs in `target/tmp/` end mid-suite) |

## Nothing suppresses, and the census is what says so

The runner prints two figures over two scopes, each with its scope inside the
line, so that neither can be quoted as though it were the other.

The first is the census proper: the count of the two tokens a failure can be
hidden behind — `-ErrorAction`'s silent one and the workflow one — over the six
files the gate set is made of: itself, the four node commands it calls
(`scripts/validate-bootstrap.mjs`, `scripts/taskctl.mjs`,
`scripts/check-non-windows.mjs`, `scripts/product-evals.mjs`) and the instrument
it calls for the parent figure (`scripts/measure-tests.mjs`). It is a pass/fail
condition: a non-zero count names the file and the count, and the run exits 4.

The second is the same two tokens counted over the tracked tree — every file
`git ls-files` lists, read through the git path the runner already resolved, so
`target/` is never walked. **It fails nothing**, and the argument is in the
runner beside it: the census's subject is the harness that produces every
reading in the record, and the tree's figure is not a finding about the tree.
What it counts is lines containing the text. This repository's own prose quotes
the token — `FINAL_REPORT.md` section 8 item 13 records the census saying "a
count of zero **about a tree that contains five**" — and a per-cmdlet probe
whose expected answer is "not found" is a deliberate local choice rather than a
hidden failure. A run reddened over that would be red on a repository that had
done nothing wrong, which is a line a reader learns to skip. What the tree
figure is for is the misreading: the zero on the first line, quoted on its own,
read as a statement about the tree.

```text
suppression census over 6 files (the gate harness): SilentlyContinue=0 continue-on-error=0
suppression across the tree: SilentlyContinue 57 line(s) in 25 file(s); continue-on-error 39 line(s) in 7 file(s); 605 tracked files outside the harness were read.
suppression-tree-note: this counts LINES CONTAINING THE TEXT and is not a count of suppressions. This repository's own prose quotes the token -- the census line above is quoted in every gate log, in `FINAL_REPORT.md` and in `progress/HANDOFF.md` -- and a per-cmdlet probe whose expected answer is "not found" is a deliberate local choice, not a hidden failure.
suppression-tree-note: it is printed so the harness zero above cannot be read as a claim about the tree. It is not a pass/fail condition on purpose -- the census's subject is the harness that produces every reading, and a figure that merges prose with deliberate local probes is a red on a repository that has done nothing wrong, which is a line a reader learns to skip.
```

Those are the lines the runner printed on 2026-09-21, taken with
`pwsh -NoProfile -File scripts/gates.ps1 -Label p17t002-worker-preflight
-PreflightOnly` on the tree at `4a0c456` and copied out of
`target/tmp/gates-p17t002-worker-preflight-preflight.txt`; they are not
re-typed. The number in the first line is `$CensusRelative.Count`, so that line
says how many files the census actually read rather than how many somebody
remembered there were — which is the difference that matters, because the
comment above that array claimed "all six" over an array of five until this row
was added and named the three node commands among the commands above it as
"four".

**The tree figure is a reading and not a constant, because the repository's own
sentences are inside what it counts.** Four readings of the same tracked tree
while this row was written: 53 lines in 25 files and 36 in 7, then 55 in 25 and
37 in 7 as the record gained sentences about the task, then 56 in 25 and 38 in 7
once this section's block was in the file, then the run quoted above at 57 in 25
and 39 in 7 once the sentence further down that re-measures the census spelled
the census line again to show what it prints when it fires. No file entered or
left the count at any of the four: each step was one more sentence quoting the
text. So a later log that disagrees with the block above on the tree figure is
not evidence that anything regressed; the question to ask is what was written.

**It reads `scripts/gates.ps1` itself too**, which is why the two tokens are
spelled in two pieces in the source: a file that spells them whole cannot
honestly count them in itself. Measured: appending one `-ErrorAction`-silent
token to `scripts/validate-bootstrap.mjs` in a clone makes the census read `1`,
name the file and the count — `suppression: SilentlyContinue x1 in
scripts/validate-bootstrap.mjs` — and exit 4. Re-measured at `P17-T002` in a
scratch tree under `target/tmp/`, because the tree line changed the code around
this condition: the same edit reads `suppression census over 6 files (the gate
harness): SilentlyContinue=1 continue-on-error=0`, names the file, and exits 4;
with the token moved into a file the harness does not call, the tree line counts
it and the run exits 0. The scratch tree was removed afterwards.

Case-sensitivity is not decoration here either. `Select-String` is
case-insensitive by default; measured at `P14-T004`, a pattern meant to count
test-binary headers also matched prose and read **140 against a true 65**. And
libtest writes `running 13 tests` in lowercase while PowerShell's `-match` is
case-blind, so a case-blind pattern reads the test count as the target's name.
Every pattern in the runner that reads a line libtest wrote is `-cmatch`, and
both `Select-String` calls that read a test log pass `-CaseSensitive` — the
header count and the `^test result:` selector the sums are taken from. That
second one costs nothing and closes a hole of the same shape: measured on four
logs, `Select-String -Pattern '^test result:'` reads 86, 88, 88 and 88 entries
with and without `-CaseSensitive`, so no figure in the record moves, and a line
of *test output* that happens to begin `Test result:` can no longer be summed
into `passed=`.

Measured again on 2026-09-21, on three logs, counting
`^\s*(Running|Doc-tests)` with and without `-CaseSensitive`:

```text
gates-p15t035-accept-test.txt     76 case-sensitive   162 case-blind
gates-p15-batch-accept-test.txt   78 case-sensitive   166 case-blind
gates-p15t037-clone-test.txt      78 case-sensitive   166 case-blind
```

The first two rows are logs in this tree's `target/tmp/`. The third was measured
in a scratch clone under `P15-T037`; the clone is removed and that log with it,
and the row stays because the measurement is what is being recorded, not the
file. `target/tmp/` is not tracked either — every log this table names is a
reading, and a reading is reproduced by running the runner, which is now
tracked.

The excess is one `running N tests` line per target binary. Nothing in the
record is affected — the count printed in a summary is the case-sensitive one,
and always was. What a case-blind pattern would change is the **target name
attached to a failure**: with `-match` on
`     Running tests\demo.rs (C:\x\demo.exe)` / `running 13 tests` /
`---- a stdout ----`, the failure is named `13 tests :: a` instead of
`tests\demo.rs (C:\x\demo.exe) :: a`.

## `SHA256SUMS.txt`: this file's own tracking, and the one step that is never two

`SHA256SUMS.txt` is a **curated** manifest over a subset of the tree, and its
digests are of the **index blob**, not of the bytes on disk —
`crates/sure-testkit/src/source_manifest.rs` carries the measurement behind
that choice and `crates/sure-testkit/tests/source_manifest.rs` is the reader.
The tool never adds a path and never removes one.

That decides what tracking this runner costs. `scripts/gates.ps1` is not one of
the manifest's 195 paths, so adding it changes no digest: measured in a clone,
staging it leaves the check reading `195 listed, 195 match the index, 0 stale, 0
not in the index`, exit 0. The files that *are* listed and that move on every
acceptance are `progress/HANDOFF.md`, `progress/state.json` and
`tasks/tasks.json`, so the regeneration is a step the acceptance commit already
has to take.

**Regeneration and staging are ONE step and never two**, in this order, and
`CONTRIBUTING.md` carries the same three lines:

```text
git add <the files you changed>                              # 1
cargo run -p sure-testkit --bin source-manifest -- --write   # 2
git add SHA256SUMS.txt                                       # 3
```

Regenerating before staging hashes the previous commit's bytes and writes
digests the next `git add` invalidates. Measured in a clone: staging an edit to
a listed file (`docs/development/WINDOWS.md`) makes the check report that one
digest stale with exit 1, and step 2 restores `0 stale` and exit 0.

## What this harness does not do

- It does not weaken an execution policy, at any scope.
- It does not add a flag to a gate command to make a reading come out green.
- It does not carry a list of known-failing test names. `red:` names what failed
  and this deliberately does not filter any of it: a suppressed name is a
  failure nobody sees again, which is worse than a count. Whether a named
  failure is new is not a property of one run.
- `scripts/gates.ps1` is not itself a row in the gate set. It *is* the gate
  set's runner, and the seventh row is `node scripts/product-evals.mjs`; the
  repository-shape rules that do run under it —
  `crates/sure-testkit/tests/source_manifest.rs`,
  `crates/sure-testkit/tests/ci_workflow.rs` and the rest — are `cargo test`
  targets, which is gate 3.

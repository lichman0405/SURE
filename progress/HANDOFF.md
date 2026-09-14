# Autonomous handoff

Last updated: 2026-09-15
Branch: `claude/v0.1-autonomous`
Progress: 28 / 166 tasks accepted. **Phase P0 complete (9/9), phase P1 complete
(11/11), phase P2 in progress (8/12).** `P2-T010` is `accepted`, on a read run,
and **nothing is `in_progress`** — the next session may start any `READY` task
without adopting an orphan. What it added is described under "What `P2-T010`
added" below, and its run is read in "Reading run `34869888350`".

The commit that follows `0907acf` is not `P2-T010`'s first work: it fixes a
defect that run `34865716315` exposed, and the reason the fix came first is that
`P2-T010`'s acceptance requires the store to open reliably, which is the thing
that run showed it does not always do.

**The branch, in order, from the last accepted task to here:** `0907acf` accepts
`P2-T007`; `3be88f1` records the run of the fix that `0907acf` follows; `4746c48`
is `P2-T010`'s implementation; the commit carrying this file is the `P2-T010`
acceptance. The acceptance of `P2-T007` is therefore **not** at the tip and never
was, so the check for it is `git show --stat 0907acf` rather than
`git log -1 --stat` — the sentence here used to name the wrong command, and a
handoff that is wrong about how to verify it is worse than one that says nothing.

**`P2-T010` follows the two-commit shape `P2-T007` used**, and for the same
reason: the acceptance cannot be written honestly until its run exists, so the
implementation is committed and pushed, the run is read, and only then is the
acceptance recorded — in `progress/state.json`, in the commit with this file.

**`P2-T007`'s implementation went in on its own commit, with its own run read
before the acceptance was taken.** `586d3a3`, run `34864498113`, all five jobs
green; `435181f` records that run. The two-commit shape was deliberate and is not
the four-commit shape `P2-T006` used: the acceptance could not be written honestly
until the run existed, and once it did, only the acceptance was left.

**Where the chain stops, stated because the paragraph here said the opposite
until it was corrected.** This file previously read *"the commit after it records
the acceptance commit's own run"* — which is a rule that never terminates, since
that commit's run would need a commit too. **The rule that is actually followed,
and was followed for `P2-T006`: every substantive commit's run is read before the
next work starts, and the run of the commit that records runs is read and reported
in the session rather than enshrined in a further commit.** `P2-T007`'s chain
therefore ends at `0907acf`, the acceptance, whose run is read in the session that
took it. The two runs that had not been recorded when the acceptance landed —
`34865169857` on `435181f` and `34865317166` on `0907acf` — are rows in the table
below, added by the commit whose own run stops the chain.

A correction to the two entries before this one: each said `progress/state.json`
records the acceptance "in the commit immediately after the one carrying this
file". Neither did — in `07e20be` and its predecessor the handoff and the
acceptance landed together, so the sentence described a procedure that was not
the one followed. Stated here because a handoff that is wrong about how to
verify it is worse than one that says nothing.

**`P2-T002` has had five follow-up commits since it was accepted — and the first
two of them exist because CI had been red since the bootstrap commit and nobody
had read it.** That is still the most important thing in this file: the local
gate set on this machine cannot see platform-gated code, `P2-T002` was accepted
while three CI jobs were failing, and the acceptance was sound only by luck. The
account is in "Continuous integration" below, and the rule that came out of it is
**a push is not finished until its run has been read**. `P2-T003` is held to that
rule: its run is a row in the same table, added by a commit after this one,
because the run does not exist until this commit has been pushed.

Primary development host: Windows 11 x64 / native MSVC.

Canonical remote: `https://github.com/lichman0405/SURE.git`
Autonomous branch: `claude/v0.1-autonomous`

## Exact current state

`node scripts/taskctl.mjs status` reports:

```
Project: SURE | status: in_progress | phase: P2
{ accepted: 28, queued: 138 }
READY: P2-T008, P2-T009, P2-T012, P3-T001, P4-T007, P4-T008, P6-T001, P6-T005,
       P6-T007, P8-T001, P12-T008, P13-T001
```

**`P2-T010` is `accepted`**, on run `34869888350`, and **`in_progress` is 0**, so
nothing is half-finished and the next session may start any READY task without
adopting an orphan. Phase `P2` is **8 of 12**; `P2-T008`, `P2-T009` and `P2-T012`
are the three that would finish it.

**The READY list went 13 → 12 and nothing entered it.** `P2-T010` left the list by
being accepted, and no task became ready in the same step — which is worth
noting because the previous two acceptances each admitted new tasks. The list is
quoted from the command rather than from a prediction, which is why a step that
changes nothing is visible as nothing rather than as an omission.

**What the acceptance tool fills and what it does not, read out of the file
rather than assumed.** On `P2-T010`, `base_sha` and `head_sha` are both `null`
and `evidence` is `[]`; `taskctl accept` sets `status`, `finished_at` and `notes`
and nothing else. **That is true of the SHAs for all 28 accepted tasks — 0 carry
a `base_sha` or a `head_sha` — but it is NOT true of `evidence` or `notes`,
and a blanket claim would have been wrong in two directions:** `P2-T003` is the
one accepted task of the 28 with a non-empty `evidence` array (three strings,
added when that acceptance was recorded), and `P0-T009`, `P1-T001` and `P1-T002`
carry no notes at all where the other 25 do. So the commits and the run for
`P2-T010` are recorded **here**, and the fields in `progress/state.json` are not
a substitute for this file — they are not even uniform across tasks.

**A correction that was made and then overtaken, kept because both halves are
worth having.** An earlier draft of this file said `accepted: 26` and "nothing is
`in_progress`" while `progress/state.json` had `P2-T006` as `in_progress` with
`finished_at: null` and an empty `notes` — the implementation commit had been
pushed and its run read, and neither of those is an acceptance. The count had
been carried forward from a summary instead of read out of the file it describes,
which is the failure mode this repository is built against. It was corrected to
`25` while the acceptance was still outstanding, and `26` was a **separate,
later** reading of the same command. The distinction is the point: a figure that
becomes true later was not true when it was written. `27` above is a third
reading, taken after the acceptance landed.

**`READY` gained `P2-T007` and `P4-T008` when `P2-T006` was accepted** — neither
could start until Cargo discovery existed. That is the dependency graph doing its
job, and it is the reason the list is quoted from the command rather than
remembered.

`P2-T007` (the component graph) is `accepted` and described in "What `P2-T007`
added" below, with its implementation run read in "Reading run `34864498113`".
`P2-T006` is `accepted` and described in "What `P2-T006` added".
The three things worth
knowing before touching any of it are that **one `Budget` serves all three
ecosystems** — and the same one is shared, so a Node-heavy project starves both
Python and Rust by call order, now measured as exactly two affected files rather
than asserted; that **a `Cargo.toml` has two sibling tables and either can be
absent**, so a virtual manifest is a project that declares a great deal; and that
**`build.rs` is a program and `src/lib.rs` is not read**, only reported as a
target at that path. Everything else on this branch is the `P2-T005`, `P2-T004`,
`P2-T003` and `P2-T002` line.

**`P2-T002` (Git project fingerprint) is now the previous session's work.** All
of it is on `claude/v0.1-autonomous` and green: `sure_core::fingerprint` with the
`git` and `digest` modules behind it, 37 new unit tests, the integration tests in
`crates/sure-core/tests/fingerprint_git.rs`, and the new
`docs/architecture/FINGERPRINTING.md`.

**The thing the `P2-T002` session could not verify locally is now verified, and
the evidence is named rather than assumed.** `a_link_is_recorded_by_its_target_and_not_by_what_it_points_at`,
`a_change_behind_an_unchanged_link_is_not_a_change` and
`a_change_to_a_file_sure_cannot_read_has_no_fingerprint` are `#[cfg(unix)]`, and
the body of the second was **rewritten in the `P2-T002` session without ever
having run on this machine** — it previously asserted almost nothing (see the
mutation section below). Windows cannot create a symbolic link without Developer
Mode or administrator rights, and both were probed and are absent. WSL Ubuntu
exists here with Git 2.53.0 but no Rust toolchain.

Run `34839532984`, on commit `c735a2f`, is green on all five jobs, and the three
tests were read out of the log rather than inferred from the job's colour:

```
test a_change_to_a_file_sure_cannot_read_has_no_fingerprint ... ok
test a_change_behind_an_unchanged_link_is_not_a_change ... ok
test a_link_is_recorded_by_its_target_and_not_by_what_it_points_at ... ok
```

The same run settles the `paths/compare.rs` split, which no local run could:
the two case-rule tests are **disjoint by platform and each runs only where its
rule holds**. macOS ran `unix::the_default_entry_point_folds_case_on_a_case_insensitive_platform`;
Ubuntu ran `unix::the_default_entry_point_folds_nothing_on_a_case_sensitive_platform`;
neither ran the other's. That is the whole point of the split, and it is now
observed rather than intended.

`9f13f0d` added a fourth Unix-only test,
`a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang`, which no local
run can execute either. Two of its constructs were compiled under `-D warnings`
on this host in isolation (`Result::is_ok_and` taking `ExitStatus::success`, and
an un-joined `thread::spawn`) precisely because "gated to another platform" is
where the last four CI failures lived. Run `34840217454` is green on all five
jobs, and the test was read out of **both** Unix logs by name:

```
test a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang ... ok
```

That is the first test in this repository whose only purpose is to prove a
project cannot make a check hang, and it has now run somewhere.

**Correction to the previous two entries, and the correction to the
correction.** `P1-T010`'s "19 test binaries" counted `store_concurrency`'s child
processes; `P1-T011` corrected the count to 15 and said "four doc-test targets
report 0". Both were true when written and both are now wrong as descriptions of
the repository: the **15 became 16, then 18, and is now 19**, and the doc-test
zeros became a one, then **three**, and are now **four** (`P2-T004` added
`discover::discover`'s). `target/tmp/count_tests.py` **used to skip the
`Doc-tests` sections entirely**, which is why the figure it printed and the
figure in the handoff disagreed by one until both were changed together. It now
counts them and prints them separately.

**Counting `#[test]` attributes does not reproduce these figures**, for two
reasons and not one: `sure-domain`'s `variants!` macro generates tests that no
attribute names, and it undercounts the suite by about twenty; and the
platform-gated tests are all counted by grep and only some of them are compiled.
Take the numbers from a run, and use the per-file *deltas* when the question is
what a commit added.

### Count the parent lines, not the `test result:` lines

**The raw number of `test result: ok` lines overstates this suite.** A workspace
run now prints **33** of them for **726** tests — 19 test binaries + 4 doc-test
targets + the **ten** child processes `store_concurrency` spawns (4 writers + 6
openers), each of which prints its own `test result: ok. 1 passed; … 6 filtered
out` into the parent's stdout. `--quiet` does not suppress that summary line —
libtest's `--quiet` drops the `running N tests` line and the per-test lines and
still prints the summary. The comment in `spawn_child` said otherwise and has
been corrected.

**So the raw line count can undercount as well as overcount, and it did.** One
captured log of this suite held **32** result lines where 33 are expected. The
missing one is identified rather than guessed: it is **`Doc-tests sure_testkit`,
a section that runs 0 tests**, whose summary line did not survive the capture.
That is why the arithmetic still looked right — a zero-test section contributes
0 to a sum — and it is the reason `count_tests.py` takes the **last** result
inside each `Running …` / `Doc-tests …` section instead of counting lines at
all. A line count is a property of the capture as much as of the suite, in both
directions: ten extra lines from the children, and one line fewer from a stream
that raced.

This matters for the record, not just for tidiness: the figure written into
`P1-T005`'s acceptance note (**408 passed**) was a raw sum of those lines and is
therefore **inflated by the child lines**. The true parent-only figure at
`P1-T005` was 396 passed / 1 ignored, i.e. 397 tests; `P1-T008` adds `sure-cli`'s
33, which was the 429 of that era. Nothing regressed — the earlier number was
counted wrong.

**`target/tmp/count_tests.py` (git-ignored) is the script that gets this right**,
and it is worth reusing rather than re-deriving. It parses one section per
`Running … (path)` or `Doc-tests …` header and takes the **last** `test result:`
inside each. Three things it had to get right, each of which produced a wrong
total first: cargo writes the `Running` markers to **stderr** and the binaries to
**stdout**, so the two streams must share one pipe with ordering preserved
(`stderr=subprocess.STDOUT` — `capture_output=True` cannot be combined with it);
`store_concurrency`'s children print `test result:` lines of their own; and
doc-tests print `test result:` with no `Running` marker, so "last line wins"
alone lets them overwrite the section before them. Five different wrong totals
came out of getting these wrong in turn — 485, 482, 137, 0, 0.

`store_concurrency` takes about a second and its children show up in the output
as lines of nine characters each. `tests/store_concurrency.rs` and
`tests/cli_contract.rs` are the only two files that spawn processes.

## What `P2-T010` added

`crates/sure-core/src/project_intent.rs`, its integration test
`crates/sure-core/tests/project_intent_ingest.rs`, `crates/sure-cli/src/check.rs`,
and the two new `Report` variants that the CLI needed to answer with. The
documentation is `docs/architecture/PROJECT_INTENT.md` (rewritten from a stub
whose sections were all still in the imperative) and a new section in
`docs/architecture/CLI.md`.

The acceptance is one sentence: *"`sure check` can receive/store a trusted
explicit goal without requiring raw transcript recording."* It has three
separable claims, and each is answered in a different place:

- **receive** — `explicit_goal(&str)` turns the text into one `Requirement` with
  the fixed identifier `goal`. The text is stored **verbatim**: not trimmed, not
  re-wrapped, not cut at the first full stop. The emptiness test trims, because
  `--goal "   "` has no words in it however it was spelled; the stored value does
  not, because trimming is already an edit.
- **trusted** — the source is `IntentSource::ExplicitUserGoal`, one of the only
  two `is_user_requirement` accepts, and `raw_retained` is true because the words
  are the user's own rather than a summary of them.
- **without requiring raw transcript recording** — nothing consults
  `requires_full_recording`, and
  `storing_a_goal_writes_no_recording` asserts the *absence* of a recording row
  through `HistoryFilter::recordings()`. That is the half a test asserting "a row
  appeared" would miss.

`crates/sure-cli/src/check.rs` is where the run happens, and three decisions in
it are the ones worth knowing about:

1. **A bare `sure check` is unchanged.** `--goal` absent means "check the
   project", which this build cannot do, so it still returns the same `NotYet` at
   status 3 and still touches nothing. Adding the flag turned no existing command
   line into a different answer.
2. **Recording a goal is `Report::GoalRecorded`, not a `NotYet`** — a new result
   shape, at status 3. The check did not happen, but the user's history changed,
   and a refusal that said only "not implemented in this build" would be true
   about the check and false about the run. Status 3 stops the script; the frame
   carries the record. Reporting 0 here would make `sure check` a green light in
   CI while checking nothing, which is the failure this program exists to find.
3. **A run that tried and could not finish is `Report::Failed` at status 5**, not
   3. The remedy for 3 is a newer build; the remedy for 5 is to look at the
   machine. `outcome`'s closed set in `docs/architecture/CLI.md` grew to include
   `failed`.

The goal is bound to a **real** project fingerprint — `project_fingerprint` with
default options — and the report leads with the kind and the digest rather than
with the identifier, because `FingerprintId` is minted per run. The consequence
for a reader is written down in `PROJECT_INTENT.md`: find the goal by project
root and kind, not by fingerprint.

### Two things `P2-T010` could not test, both recorded rather than papered over

**The happy path is not covered as a process.** `cli_contract.rs` runs the
binary, and it runs `sure check --goal ""` and nothing more: a goal *with words*
would be written to the store the developer's own machine really uses, and on
Windows that location comes from `SHGetKnownFolderPath`, which ignores
`LOCALAPPDATA` — so no environment variable can point a test at a scratch
directory. A test that did it would put an invented requirement into somebody's
history and look exactly like a green one. The code under that path is driven by
`src/check.rs`'s unit tests against locations they name, and by
`sure-core/tests/project_intent_ingest.rs`. `docs/architecture/CLI.md` states the
gap in those words.

**One code path's defence cannot be observed at all.** The early return in
`check::run` — "a bare `sure check` does not look for its files" — is a promise
about a machine where SURE has *no* data directory. Removable, the answer a user
gets is identical and the difference is a store created on a machine that did not
have one. No test in this repository can see it, for the same reason as above.
It is the second mutation in `mutate12.py`'s declared-unobservable set, and it
closes the same day a caller can choose where SURE keeps its files.

### The mutation run wrote six rows into the real store, and that is the same gap seen from the other side

**The gap above is not only a missing test. It has a cost, and this run paid it.**
`cli_contract.rs` runs the real `sure.exe` against the real locations, which on
Windows means the developer's actual `%LOCALAPPDATA%\SURE\sure.db`. In the
unmutated build that is harmless, because the only goal it passes is `--goal ""`
and an empty goal is refused before the store is opened. **The mutation that
deletes that refusal therefore does not merely fail a test — it writes into
somebody's history.** It did:

| id | kind | document | project_root |
|---|---|---|---|
| 1, 2, 3, 4, 5, 6 | `project-intent` | `{"id":"goal","raw_retained":true,"source":"explicit_user_goal","text":""}` | `C:\Users\lishi\code\SURE\crates\sure-cli` |

Six rows, in three pairs a few hundred milliseconds apart, all inside a
254-second window (`1789403446` – `1789403700`, ms since epoch). `text` is empty
in every one — a shape no shipped code path can produce, which is what identifies
them as the mutation's rather than a user's.

**Two things were measured rather than assumed, and the second is the one that
matters:**

- **An unmutated suite run leaves the store byte-identical.** Taken around a
  single `cargo test -p sure-cli --test cli_contract`: 6 rows before, 6 rows
  after, the same SHA-256 over the row contents, and the same MD5 over the file.
  So the shipped tests do not pollute it, and the claim in `docs/architecture/CLI.md`
  about the gap is accurate as written.
- **The file's hash is not a before/after comparison across sessions.** The hash
  recorded in the previous session's handoff was compared against the hash today
  and differed — and that comparison was meaningless, because it straddled a
  migration and six writes. This is the second time this file has had to say that
  a hash taken at two different times is not a measurement; the pairing has to be
  taken around one command.

**The rows are left in place, and removing them is the owner's call.** They are
false records in a history file that has no backup, and deleting them is not
reversible; the exact statement is
`delete from records where id in (1,2,3,4,5,6)` against that database, which the
owner may run or not. Leaving them also keeps the evidence, which is why the
default here is to leave them. **The durable fix is the one already named: the day
a caller can choose where SURE keeps its files, this test stops being able to
touch the real store at all** — and until then, any mutation that removes the
empty-goal refusal will do this again, which is worth knowing before the next
mutation run rather than after it.

### Two things `P2-T010` observed and did not fix

Neither is a defect claim; both are things the next reader will meet.

1. **`FingerprintId` is minted per call while `Evidence::is_fresh_for` compares
   ids.** `ProjectFingerprint::git` and `::content` call `FingerprintId::generate`
   every time, so an unchanged project gets a different identifier on every run,
   while `sure_domain::evidence::Evidence::is_fresh_for(current)` asks whether two
   *identifiers* are equal. If that predicate is ever the thing that decides
   whether stored evidence is stale, it will answer "not fresh" for evidence
   about a project nobody touched — the failure mode `choose.rs` and
   `FINGERPRINTING.md` both argue against. Nothing calls it with a real
   fingerprint id yet, so this is a question to answer rather than a bug to fix:
   **is a fingerprint id meant to be stable across runs, or is `kind` + `digest`
   the identity and the id only names one computation?** `P2-T010` assumed the
   second and reports `kind` and `digest`, keeping the id to the machine frame.
   `docs/architecture/PROJECT_INTENT.md` records the same assumption.
2. **`HistoryFilter` has no `project_root` filter.**
   `sure_core::store::HistoryFilter` filters by project fingerprint and by record
   kind, and a goal is stored against a project *root*. So a reader asking a
   shared user-level store for one project's goal must fetch by kind and filter
   by root itself. Nothing in this build reads a goal back at the user level —
   `sure check --goal` only writes — so the gap is recorded rather than worked
   around, and the task that first reads one owns closing it.

### The `P2-T010` mutation run, and the two holes it found

`target/tmp/mutate12.py` (git-ignored), 23 mutations over three files: **21 of
21 observable mutations caught by a failing test, 2 declared unobservable and
missed as declared, 0 skipped, 0 that failed to compile.**

The families are the three claims above plus the one about what a run *says*, and
the third is where the value was. Eleven mutations are false-green shapes —
`sure check --goal` reporting `ok`, exiting 0, filing a recorded goal as an
answer about the project, reporting a summary of the goal as the goal itself.
Each fails three tests at once, which is the point: the exit status, the outcome
word and the human sentence are three independent renderings of one decision and
the suite holds all three.

Two mutations came back green on the first run, and both tests were written
afterwards:

- **"the goal is summarized into one sentence before it is stored"** —
  `Requirement::text` is documented as *"normalized to one sentence where
  possible"*, so cutting at the first full stop is the most plausible way to get
  the no-summarizing rule wrong. Every goal in the test had no full stop in it, so
  the mutation changed nothing. A user writing a goal writes several sentences
  when it takes several to say what they want — which is exactly when losing the
  rest matters most. Two multi-sentence goals are in the test now.
- **"the store is opened before SURE knows whether it has anything to write"** —
  and the reason it was green is the more useful finding.
  `a_project_that_cannot_be_read_leaves_no_store_behind` used a **relative** root,
  and `Store::open` refuses a relative root through `Paths::ensure_outside` as
  well, so the module that opened the store first passed the very test written to
  catch it. The test now also uses an absolute path that does not exist — the
  ordinary mistake of a mistyped path — which only the fingerprinter refuses.

The third rule of the harness earned its place twice: one mutation was reported
`SKIP` because its anchor did not match (the formatter had reflowed the line), and
one was reported `BUILD` because it made a `match` non-exhaustive. Neither
counted as a catch, which is what those verdicts are for — a mutation that was
never applied, or that stopped the code compiling, says nothing about whether a
test would have noticed the behaviour.

**A fourth thing this run did was not a test result at all.** The whitespace
mutation — the first in the list, and the one whose anchor is the refusal this
module exists to state — made the process-level test in `cli_contract.rs` write
six empty-text rows into the developer's real store, because that test runs the
real binary against the real locations. **The mutation harness has a side effect
outside the repository, and nothing in this file said so before.** The rows, the
reason, the measurement showing the unmutated suite is clean, and the one-line
statement that removes them are in "The mutation run wrote six rows into the real
store" above. Worth knowing before the next mutation run rather than after it.

## What `P2-T007` added

`crates/sure-core/src/components.rs` (a module of `sure-core`, beside `discover`)
and `crates/sure-core/tests/components_graph.rs`, plus
`docs/architecture/COMPONENT_GRAPH.md`.

`ComponentGraph::of(&Discovery)` is a **view** — it opens no file, and
`the_component_graph_opens_no_file_and_starts_no_process` enforces that against
the source rather than trusting the module comment that says it. It produces a
list of components (the root, then every directory a workspace declaration
named), the containment edges between them, and one `EcosystemResolution` per
ecosystem.

The task's second acceptance criterion — *"unknown stack details remain
inference"* — is carried by the type rather than by a convention:

- `Component::manifests` holds **one `ComponentManifest` per ecosystem**, never a
  merged verdict. Node and Rust can name the same directory, one may have read a
  manifest there and the other not, and a single field would have to pick — losing
  a fact silently in one direction or the other. Keeping both removes the merge
  decision rather than guarding it, and `Component::stack()` is a derivation over
  the list.
- `Stack` is `Read` / `Partial` / `Unknown`, with **no `Option` anywhere**. There
  is no shape a caller can render as "no stack", and no plain value it can render
  as a fact.
- `ManifestReading` has five arms because discovery established five different
  things. The pair most easily collapsed by accident is `NotOpened` (*there is a
  `package.json` SURE did not open*) versus `NoManifest` (*there is nothing
  there*); collapsing them tells a reader either that a file is missing when it
  is not, or that a file was read when it was not.
- `Members` has a `NotRead` arm that **must not be rendered as "no members"**,
  and `Members::is_known_single()` is the single question a report asks before
  saying "one component". It returns `false` for `NotRead`.

Three things worth knowing before touching it:

1. **`NotRead` is returned for every Python project.** `python.rs` reads no
   member list at all — `[tool.uv.workspace]` and `[tool.pdm.workspace]` are not
   parsed — so SURE cannot tell a one-package Python project from a fifty-package
   one. This is gap 8 of `docs/architecture/ECOSYSTEM_DISCOVERY.md`, and the
   component graph is where it would otherwise become a false claim. A Python
   monorepo therefore reports as **one component with a caveat**, and the caveat
   is in `plain_description` so a caller that never reads `resolution` still
   cannot lose it.
2. **`contains` is not a dependency graph.** `"@app/ui": "workspace:*"` is a
   request to a resolver SURE has not run. Only containment (a fact about paths)
   and declaration (a fact about files read, carrying its `Source`) are here.
3. **A Rust member that `exclude` names is still a component.** Discovery reports
   those in `Workspaces::excluded_members` and deliberately does not subtract
   them; the graph inherits that and offers no way to see which they are. That is
   recorded as gap 1 in `COMPONENT_GRAPH.md`.

The order of `components` is `Path`'s own order — component by component, so the
root is first. An earlier draft sorted by depth and then by path, and the depth
key was removed rather than tested: every fixture was one level deep, so the
mechanism was doing nothing any test could see.

## What `P2-T006` added

`crates/sure-core/src/discover/rust.rs` (2885 lines, 30 unit tests) and
`crates/sure-core/tests/discover_rust.rs` (24 tests, the new 21st test binary),
plus `crates/sure-core/src/discover/pattern.rs` — the member-pattern expansion
lifted out of `node.rs` so two ecosystems cannot come to expand a `*` two ways.
`discover/mod.rs` gained `Ecosystem::Rust` and `Findings::Rust`, and
`MemberManifest` moved up so both Node and Rust name one type; `node.rs` keeps a
re-export so `node::MemberManifest` still names what it always named.
`docs/architecture/ECOSYSTEM_DISCOVERY.md` gained a Rust half, its enforced-by
rows and five new gaps (11–15).

**The one fact the module is arranged around: `[package]` and `[workspace]` are
sibling tables of one document and either can be absent.** A `Cargo.toml` with
only `[workspace]` is a *virtual manifest*, and it declares the members, their
shared dependencies and their shared lint levels. So the workspace tables are held
on `Manifest` and **not** on `PackageSection`, and `Manifest` is a separate type
for exactly that reason. Hanging them off the package would drop a virtual
manifest's entire workspace — the case such a manifest exists for. This was the
first draft's bug, it was designed out rather than discovered, and mutation 24
reverts it and is caught. A third fact is kept apart from both: `[dependencies]`
is meaningful only where `[package]` exists, because Cargo refuses a virtual
manifest that declares dependencies.

**The same split decides the commands, and this is the second designed-out bug.**
`command_for` gates on `project.manifest.manifest()?` — the *document* — and
deliberately not on `package()`. Gating on the package would withhold every
command from a workspace root, which is a project `cargo build` acts on and
builds every member of. Mutation 23 deletes the gate and is caught.

The rest, briefly:

- **`exclude` is not subtracted from `members`.** Whether a directory named by
  both is a member is Cargo's rule and SURE has not read it, so
  `Workspaces::excluded_members` carries the overlap and neither list is applied.
  Reporting it is reversible by a reader who knows the rule; silently honouring it
  would be SURE asserting a rule it cannot cite.
- **The toolchain pin is read by the file's name, not by what parsed.**
  `rust-toolchain.toml` holds a `[toolchain]` table; the bare `rust-toolchain` is
  not TOML and usually holds one token. A `.toml` that carries no `[toolchain]`
  table is `WrongShape`, a `.toml` that does not parse is `NotParsed`, and only a
  file that is not TOML falls through to the bare-channel reading. This is the one
  place a filename changes an answer, and the doc says so.
- **Seven conventional targets**, reported as what is *at* the path and never as
  what the file contains. `build.rs` is the one to pause on: Cargo compiles and
  executes it before the crate, which makes it the only file in a Rust project
  that runs arbitrary code at build time. It is reported as a `BuildScript` target
  and nothing here executes or reads it. `src/lib.rs` is reported and never read —
  this is the one place the module leans on a Cargo convention rather than on a
  declaration, and it is gap 11.
- **`Requirement` is four facts, not an `Option`.** `Stated`, `FromWorkspace`,
  `Unstated`, `NotReadable` — `foo = { workspace = true }` is not a version in
  this file, and a spec SURE cannot read is not a dependency that is not there.
- **`grade` is the single place a level and a reason are decided together**, and
  `is_absent()` is true only for the `Absent` arm, so the one question a caller
  may collapse is the one that collapses to false for a file that is there but
  unread.
- **One `Budget` serves all three ecosystems**, and the same one is shared:
  `discover()` builds one and passes it to `node::look`, then `python::look`, then
  `rust::look` in `Ecosystem::ALL` order. A Node-heavy project starves both later
  ecosystems by call order and not by anything their modules do. Recorded in the
  doc under "What is bounded" and now **measured rather than asserted**: with
  `max_manifests(1)` the affected files are exactly two, `pyproject.toml` and
  `Cargo.toml`.

### What the second `P2-T006` commit added, and the two ways its own tests were wrong

Commit `9c931d0` was green and its run was read, and `P2-T006` was **not** accepted
on the strength of it. A background security review flagged
`crates/sure-core/src/discover/pattern.rs`, the module P2-T006 had just lifted out
of `node.rs`. Reading it independently found the containment rule sound — no
traversal, no filesystem reach, no unbounded expansion — and found the real,
actionable gap underneath: **`pattern.rs` had no tests at all**, at 209 lines and
9 tests' worth of behaviour, and it is the one module in the tree where a
*project's own text* decides which directories SURE then reads. The same rule was
reachable through three callers, so it was checked three ways and stated nowhere.

Nine tests were added, plus a fourth family of mutations (28–34) anchored on
`pattern.rs`, because a test that no mutation can break is a test this repository
does not count. The rows the new tests pin went into the doc's enforced-by table,
and the index was re-checked mechanically: **88 backticked identifiers, 87 of them
`#[test]` functions and one a deliberately named function, 0 found nowhere.**

**The first version of those tests turned the suite red, and the mutation run is
what said so.** The scratch helper used the process id to make its directory
unique — the idiom this file had been told to use — and
`discovery_runs_none_of_the_scripts_it_reads` greps the *text* of all six files in
the module tree for the process module's name. It reads `#[cfg(test)]` code too.
The test was failing before any mutation was applied, which means **every
`CAUGHT` in that run was unattributable** — a mutation cannot be credited with a
failure that was already there. Had the suite been run only as
`cargo test --lib discover::pattern`, which is what a person checking their own
work would run, all nine tests would have passed and the branch would have been
pushed red.

The fix is in the test and not in the check, because the check was right about the
file. Uniqueness now comes from `create_dir` failing rather than from a name:
`AlreadyExists` means try the next number. That is strictly stronger than the
pid — the pid is unique within one run and says **nothing** across two, so a name
built from it collides with a previous run's directory when the OS hands out the
same id, and the test then reads a fixture some earlier run had already written
into. `create_dir` refuses to adopt a directory that exists, so a stale path is
skipped whatever else is running.

**And it then turned red a second time, on the sentence explaining the fix.** The
sentinel searches raw text, so the doc comment saying "no `std::process` here,
deliberately" tripped the very check it was describing. A **mention is not an
ability**: the test now drops whole-line comments before searching, so only lines
that can hold code are read. Only whole-line comments — a trailing comment on a
line of code is still searched — and the change is documented at the check, since
the alternative was a trap that would fire on the next author who wrote a sentence
about the rule.

Neither of these was found by reading the diff. Both were found by running
something that was expected to pass.

### Two things `P2-T006` found and fixed rather than recorded

**A real gap between `P2-T005` and `P2-T006`.** The "discovery must not start a
process" source grep in `discover_node.rs` named only Node's three files
(`discover/mod.rs`, `discover/node.rs`, `discover/read.rs`). `python.rs` therefore
went unchecked for a whole task, and `rust.rs` would have gone unchecked after
this one. It now names all six files in the module tree, with a comment saying
why: the claim is about **discovery**, and a check that named one ecosystem's
files would have let the next one shell out unnoticed.

**Two tests named in the enforced-by table that do not exist.**
`a_project_that_is_every_ecosystem_at_once_is_reported_as_all_of_them` (the real
name says `a_directory_that_is…`) and
`a_workspace_inherited_dependency_is_read_as_one_that_states_a_version_here`
(invented outright, replaced with the real
`a_dependency_sure_cannot_read_is_not_a_dependency_that_is_not_there`). The check
was mechanical rather than by eye: extract every backticked identifier from the
table and require `fn <name>` to exist in the corpus. It now reports **79 names,
0 missing**. A documented index that names a nonexistent test is worse than no
index — a reader who trusts it believes a property is pinned when it is not.

## What `P2-T005` added

Python discovery, as a second module beside the Node one. Five files carry it:
`crates/sure-core/src/discover/python.rs` (2863 lines, 25 module unit tests),
`crates/sure-core/tests/discover_python.rs` (31 tests, the new **20th** test
binary), `mod.rs` (`Ecosystem::Python`, `Findings::Python`, the wiring),
`read.rs` (`read_text_file`, and `to_json` for the TOML conversion), and
`scan/ignore.rs` (`.tox`, `.nox` and `.eggs` as vendored). `toml` moved from a
test-support dependency of `sure-core` to a real one, which the workspace
manifest explains: it is deliberately **not** deserialized straight into
`serde_json::Value`, because `toml` represents a datetime as
`{"$__toml_private_datetime": ...}` — a table the project did not write, under a
key that looks like project data — and `inf`/`nan` as `null`.

**The rule is the same one `node.rs` is built on: not there is never
there-but-unreadable.** `ManifestState`, `ReadFile` and `UnreadReason` are enums
and never `Option`s, and `read_manifest` pairs the reader with the converter so a
shape failure cannot reach the state without reaching `Discovery::unread`.

**Two facts about this module are load-bearing and easy to get wrong.**

*One `Budget` serves both ecosystems.* `discover()` builds one and hands it to
`node::look` and then to `python::look`, in `Ecosystem::ALL` order. A project
with 512 Node manifests therefore leaves nothing for Python, and its
`pyproject.toml` is `OutOfBudget` rather than read. That is the intended reading
of a limit on how many manifests SURE reads in one discovery, but it is a
consequence of the call order and not of anything Python's module does. It is
written down in `ECOSYSTEM_DISCOVERY.md` rather than left to be discovered.

*`setup.py` is a program, so it is never read.* It declares its dependencies by
executing Python, so it is `unread_legacy` — a name carried through to the
result — and a project whose only manifest is a `setup.py` gets `InspectOnly`,
which says exactly that. `discovery_runs_nothing` writes a `setup.py` and a
`conftest.py` that would each leave a file behind if executed **or imported**,
runs discovery, and requires that no such file appears; it also lists the
directory, so the assertion is not only about the two names it could think of.

**A requirements file is the weakest evidence and is never a disagreement on its
own.** pip, uv, poetry and pdm all read `requirements.txt`, so a `uv.lock` beside
one is the ordinary shape of a project that moved to uv. Reported as a
contradiction it would be a false alarm, and a false alarm beside a real one is
how a reader learns to ignore both. It is still collected and still reported, as
the third tier, where it decides only when nothing stronger is present.

**Requirement names are parsed and two rejections stop a name being invented.**
A name immediately followed by `:` or `/` is not a name, which is what stops
`https://example.invalid/pkg-1.0.whl` being read as a dependency called `https`;
and a run ending in a non-alphanumeric is not a name, because `foo-` and `foo.`
are prefixes. Every line of a requirements file lands in `requirements`,
`directives` or `comments`, and a test asserts the three add up to the file's
line count.

**The mutation run found one real hole in a suite that was already green.**
Changing the support level of the "recognised, but nothing SURE reads declares
anything" arm from `InspectOnly` to `Generic` passed every test — so a project
SURE had read nothing from (a bare `uv.lock`, or a bare `.python-version`) was
being reported as a project SURE fully understands. That is the false-green
direction, and
`a_project_with_no_manifest_sure_can_read_is_not_called_fully_understood` closes
it and asserts the reason carries no project text. `target/tmp/mutate9.py`
applies **23** mutations, **22 caught** — the figures are from a re-run at the
end of this session, and the count written here first was 22 and 21, wrong by one
both times, see the mutation section — and the one that is not caught is recorded
in the script with its reason and the verdict is right: it is the same standing
as `FINGERPRINTING.md` gap 8.

**Two things this work falsified, fixed rather than left.**
`a_directory_with_no_node_files_in_it_is_not_a_node_project` asserted the exact
list of ecosystems, which broke the moment Python was added: it was a fact about
the build that the test had no business pinning, and it now asserts
`Ecosystem::ALL` plus that Node is one of them. And the previous handoff claimed
`crates/sure-testkit/tests/repository_shape.rs` pins a dependency's category —
**that is false**, and the correction is written into the "Next concrete action"
entry where the claim was made.

## What `P2-T004` added, and the defect its verification turned up

`crates/sure-core/src/discover/` is new: `mod.rs` (the ecosystem-agnostic shell
and `discover()`), `read.rs` (one reader for one file, and the reasons a file can
be unread), and `node.rs` (everything Node-specific). 33 integration tests in
`tests/discover_node.rs` — the new **19th** test binary — 24 unit tests in the
module, and one doc test. That is +58, and the workspace total went **668 → 726**
with the arithmetic closing exactly. `docs/architecture/ECOSYSTEM_DISCOVERY.md`
is the authority; `P2-T004`'s acceptance note carries the summary.

**The rule the task is built on: the four questions a manifest can be asked are
answered with an enum, never with an `Option`.** A manifest that is not there, a
manifest that is there and could not be read, and a manifest that was read are
three different worlds, and `None` collapses the last two into "no
dependencies" — a package whose manifest could not be read would be reported as
a package with nothing in it. So `ReadFile` is `Absent`/`Parsed`/`Unread`,
`ManifestState` is `Read`/`Absent`/`Unread`, `MemberManifest` is
`Present`/`Absent`/`NotReadable`, and `UnreadReason` names all seven ways a file
reaches the third state rather than one. `read.rs` is the only place that decides
which, and that is what makes the rule structural rather than a convention: a
caller that wants to confuse the two states now has to write the confusion out
loud, in a match arm, where a reviewer can see it.

**The package manager is not "the one with a lockfile".** Four managers are
recognised — npm, yarn, pnpm, bun — and each can be evidenced three ways of
different strength: a `packageManager` declaration, a lockfile, and an `engines`
range. `Managers::agreed()` returns the one manager every source points at, and
`None` when they disagree. Two lockfiles, or a declaration that contradicts the
lockfile beside it, are reported as a `Disagreement` rather than resolved by a
precedence rule the reader cannot see. A manager SURE does not recognise is not
reported as one it does.

**Workspaces are resolved against the directories that are really there.** The
root is never a member of its own workspace. A pattern that would reach outside
the project is refused; a pattern that names nothing is reported as naming
nothing, because the alternative is a workspace list that silently covers less
than it claims; and a list that was cut short says so. `packages/*` expands one
level, and each component in turn; a pattern relying on brace expansion or
character classes — which npm does not support either — is reported as
`UnsupportedPattern` rather than approximated.

**Scripts.** 8 conventional roles, each looked up across the spellings projects
actually use, and `command_for(manager, role)` renders the command the declared
manager needs, so the report never proposes `npm run` for a pnpm project. A
script that is present and has no command does not read as a script that is
absent — the same distinction as the manifest, one level down.

**Frameworks and tooling are one 67-row table across 11 roles.** The package
name stored in a `Tooling` always comes from the table and never from the
manifest, so a project cannot get a role by naming itself something. Two rows
carry `@biomejs/biome` — it is a linter and a formatter, and one dependency holds
both roles — which is why `tooling_of_role` returns an iterator rather than one
entry.

**`discovery_runs_none_of_the_scripts_it_reads`.** This module executes nothing.
That is the property the module is shaped around rather than a promise added
afterwards, and it is the same line `FINGERPRINTING.md` and `EXECUTION_SAFETY.md`
draw.

### The mutation run, and the one hole it found

`target/tmp/mutate8.py` applies **27 mutations; 27 were caught.** Two are
deliberately not in the script and it records why: a tool-name substitution is
not expressible, because `collect_tooling` only pushes when the name already
matches, and the tooling sort is not observable from a single run. Three anchors
had to be made more specific after the first run — one had matched two identical
probes (`if matches!(tree.probe(…), Probe::File(_))` at the lockfile and at the
typescript config, in the same shape), one removing a `seen.insert` left the
binding unused and so did not compile, and one was a no-op.

**The hole it found was in a suite that was already green.** `Package::dependencies_not_ranges`
— the field that keeps *declares `left-pad: ^1.3.0`* and *declares `jest: [29]`*
from reading alike — had **no test anywhere**. The mutation that deletes it was
MISSED, which is what surfaced it. Two tests close it, and the second is the one
that matters: a name declared with no range in **two** sections must be named
**once**, which is what makes the `dedup()` beside the `sort()` load-bearing
rather than decorative. This is the latest of several times on this branch that a
mutation found something no test reached — `P2-T001` found two, `P2-T003` found a
test that was reaching the wrong branch. It is the cheapest way to find one, and
it keeps being worth running.

### What `P2-T004` does not establish

The five gaps `ECOSYSTEM_DISCOVERY.md` records, none of them closed by this task:
the byte budget has only been exercised where the manifest itself crossed it (the
`+ 1` read that catches a file growing *during* the read has never been seen to
fire); `Unreadable` is reached by a missing path and by an interior NUL, never by
a file the OS refuses to let this user read; the two spellings `package.json` and
`Package.json` in one project is constructible only on a case-sensitive
filesystem, so the Linux and macOS jobs are where it would run; workspace pattern
expansion is one component at a time and brace expansion is reported as
`UnsupportedPattern` rather than approximated; and the link-at-the-manifest case
is a **directory** link, because `mklink /J` needs no privilege here and a file
symbolic link does, so the file-link arm is one code path by argument rather than
by test. **CI has now run on `P2-T004` and falsified something outside this
list** — not one of these five gaps but a claim that there were no
platform-dependent surfaces at all. See "Reading run `34850549120`" below, which
also records that the `#[cfg(unix)]` arms of this task remain uncompiled and
unrun here.

## The defect `P2-T004`'s verification found: five scratch paths every run reused

Found while chasing a suite that failed roughly one run in eight, always in a
different test, always clean on a re-run. It is fixed in `0a577ca` and it is the
reason that commit exists separately.

**What was wrong.** Five test helpers cleared a scratch directory with
`let _ = std::fs::remove_dir_all(&path)` and then used the path as though it were
empty. On Windows that deletion can fail — the previous run's database is still
open — and the discarded error turned the clear into a wish. The test then read
somebody else's records as its own.

**The instance that is proven, and the proof.** `tests/store_concurrency.rs`
reported **200 records where 100 had been written**, and reported a lost write.
Both reports were wrong: the file had never been cleared, so the count was the
old run's rows added to the new ones, and the "lost" write had been there all
along. Established by holding a handle open across the clear and
watching it happen deterministically, not by argument. **The wrong number is the
point** — the suite did not merely fail to see the truth, it stated the opposite
of it, which is the kind of report this repository treats as worse than an error.

**The four that are not proven.** All five helpers now name their scratch
directory after `std::process::id()` — the convention `config/authority.rs`,
`config/mod.rs` and `discover_node.rs` already followed — and all five keep a
loud backstop for the case where the id has been reused. But **only the first is
shown to be that fault**, and the comments in the other four say so:

- `crates/sure-core/src/store/mod.rs`'s test module — the same shape as the one
  above, in the store's own unit tests, not separately reproduced.
- `crates/sure-core/src/doctor.rs` — seen to fail under a loaded run with
  `the store was readable: NotCreated` and `Unreadable` with `os error 5`, and
  those stopped once the path was unique; **that is not proof the path caused
  them**.
- `crates/sure-core/tests/doctor.rs` — this one fails about **once in ten
  whole-workspace runs and never once in twelve runs of its binary alone**. The
  only code in the repository that deletes that path is `scratch` itself.
  Removing a shared resource is not the same as repairing a proven fault, and the
  comment in the file says exactly that.
- `crates/sure-testkit/tests/integration_thinness.rs` — failed with
  `Os { code: 3, kind: NotFound }` on a write into a directory `create_dir_all`
  had just made. **The obvious explanation did not survive a probe**: 3000 rounds
  of that exact shape — fixed name, create, write, remove, repeat — failed zero
  times, and 3000 with a unique name also failed zero times. So the pending-delete
  story is falsified as far as this machine can falsify it, and the comment says
  the cause is not known rather than naming one.

**One fix was tried and abandoned, and it is worth recording.** The first attempt
kept the fixed path and made the failed clear loud instead of silent. Failures
went from ~5–9 per ten runs to **9–18**: with the clear now fatal, a hidden lock
made the test stop instead of quietly reading stale data. Making the symptom
louder without removing the shared resource made the suite worse. Uniqueness is
the substantive fix; the backstop is only there for the case uniqueness cannot
cover.

**Gates for the fix and for `P2-T004` together:** `cargo fmt --all -- --check`
clean; `cargo clippy --workspace --all-targets -- -D warnings` clean;
`cargo test --workspace` **726 passed, 0 failed, 1 ignored** across 19 test
binaries and 4 doc-test targets; `node scripts/taskctl.mjs validate` = 166 tasks;
`pwsh scripts/Preflight-Windows.ps1` passed. The flake evidence is
**16 consecutive piped workspace runs with 0 failures**, under the exact pipeline
that failed 8 of 8 before the change — recorded as `n` runs clean, not as "fixed",
because a flake that has stopped appearing has not thereby been explained.

## What `P2-T003` added, and the false green it found in its own tests

The non-Git fingerprint, and — the part that is a decision rather than an
implementation — **which of the two kinds a project gets.** Three new modules and
an extraction, in `7ce90bf`:

- `content.rs` — the manifest. It walks with `crate::scan`, the same tables and
  the same comparison the checks use, so "generated and vendor churn is excluded"
  is one rule and not a second list that can drift from the first.
  `the_excluded_list_is_the_scans_and_not_a_second_copy_of_it` pins the agreement
  rather than the list. Every limit — `max_depth`, `max_entries`, `max_files`,
  `max_bytes` — is an error and never a digest over the part that fitted.
- `choose.rs` — the one place the kind is decided, so that the decision is a
  sentence somebody wrote rather than an accident of which function a caller
  reached for.
- `read.rs` — `Reader`, `Contents`, `file_kind` and `display_path`, moved out of
  `git/mod.rs` unchanged. Two implementations of "what is at this path" would be
  two answers to a question that has one, and the way they would diverge is not
  symmetric. Verified behaviour-preserving by the Git kind's tests, which did not
  change.
- `tests/fingerprint_content.rs` — 23 tests (25 on Unix; two are `#[cfg(unix)]`),
  plus one lib test in each of `choose.rs` and `content.rs`.
- `docs/architecture/FINGERPRINTING.md` — a content-fingerprint section, the
  dispatch rule and its table, and **gap 8**.

### The dispatch rule, and why the obvious one is wrong

**A project is fingerprinted by Git when it is at the root of the working tree
that contains it; otherwise by content.** `git rev-parse --show-prefix` is empty
exactly then.

"Is this directory inside a repository?" is the check anyone would write first,
and it answers *yes* for a directory one component deep in somebody else's
checkout. The Git kind digests `HEAD` on purpose, so for a subdirectory that is
exactly backwards: **a commit anywhere else in the repository moves `HEAD`, so
the subdirectory's fingerprint moves although not one of its files changed.**
Evidence marked stale over and over for a project nobody touched is how a person
learns to stop reading the word "stale".

That is not argued, it is measured, in both directions, by
`the_git_kind_moves_for_a_commit_the_project_is_not_part_of_and_the_content_kind_does_not`
— which also asserts that the Git kind *does* still move, so the reason cannot
quietly become folklore that outlives its truth.

**`GitUnavailable` stays an error and is deliberately not a fallback to content.**
The kind a project gets has to be a function of the project and not of the
machine. Falling back would mean one unchanged directory produced a `Git`
fingerprint on a laptop and a `Content` fingerprint on an agent without Git;
`ProjectFingerprint::matches` compares the kind, so every stored result would be
stale on the other machine and a project checked in both places would never agree
with itself. A caller who wants the content manifest anyway calls
`content_fingerprint` directly, which is the escape hatch and is explicit on
purpose.

`choose.rs`'s module test is the only test in the repository that can reach that
branch: the other three outcomes are decided by the *project* and a fixture can
build each, but this one is decided by the *machine* and needs a Git that is not
installed. `fingerprint_with` is `pub(crate)` for that reason, so the test lives
in the module rather than beside the other chooser tests.

### A false green this task found in its own tests

**The test named `a_project_in_no_repository_at_all_is_fingerprinted_by_content`
used a fixture under `target/` — which is inside the SURE checkout and therefore
inside a working tree.** It exercised the "inside somebody else's repository"
branch and never the "no repository" one, and it passed. Nothing about its
assertion was false, and nothing said the fixture could not reach its case.

The mutation *"a directory in no repository is refused rather than read by
content"* was **MISSED by every test in the suite**, which is what surfaced it.
Fixed by `Fixture::outside_any_repository` (the system temp directory) plus a
`git_prefix()` helper that asserts the premise **with Git directly** — a premise
checked with the code under test is not a premise. The same premise assertion was
added to the "inside somebody else's repository" test, where it immediately
caught a second thing: the comparison was against `Some("inner/")` while Git
prints a trailing newline, so the reading was wrong and the assertion was right.

### Two properties that no test holds, recorded as gap 8 rather than left looking covered

Same treatment `--includes` got in the previous session, for the same reason.

- **The sort before hashing.** Removing it passes every test. The digest is a
  value, and the sort's whole purpose is to make it independent of the order the
  walk happened to produce — one run sees one filesystem's order and no other, so
  there is nothing a test could compare two of. The Git kind's sort has the same
  status, which is why `mutate6.py` does not mutate it either.
- **The domain tag.** Setting it to the Git kind's also passes every behavioural
  test, because the two digests cannot collide even with one tag — the Git kind
  opens with `head` and the content kind with `manifest`. So the tag is belt and
  braces over the field structure rather than the thing that keeps the kinds
  apart, and an earlier comment in this task said the opposite until the mutation
  showed it. What pins it now is a constant assertion, and what that assertion
  protects is the *decision*: the tag is part of the format of every fingerprint
  already stored.

### `P2-T003` mutation results

`target/tmp/mutate7.py` (git-ignored, 16 mutations): **all 16 caught**, and the
two link mutations reported `BLIND` on Windows rather than omitted. It found
three boundary-weak tests as well, which are now pinned from both sides: the file
limit asserts three files under limits of three *and* two, and the byte budget is
finally shown to be spent by the whole manifest rather than by each file.

It also found that **`read.rs` is reached through the Git kind** — the walk inside
a nested directory is only ever reached there, because a nested checkout Git
refuses to descend into is one untracked *path*. Two mutations were reported
MISSED until the script was made to run `--test fingerprint_git` as well, and
that is why its test command names both binaries.

## What the filter hardening added, and the two claims it corrected

**A security review of `9f13f0d` — run automatically, and arriving as a
background notification rather than from a person — found that fingerprinting a
project could still execute a program the repository chose. It was right.** The
commit `5705444` is the fix. No user input was involved and none is implied.

The review's one-line finding (`subprocess-rce-via-untrusted-git-config`,
"incomplete fix") was checked rather than taken on trust, and the check is what
made the fix designable: a marker program behind a tracked `.gitattributes` plus
a `filter.<n>.clean` setting in the repository's own configuration is executed
by a plain `git status` **with nothing modified**. The control that proves the
fixture really fires is what makes the negative assertion mean anything, and the
first version of the probe was wrong in exactly that way — it reported `RAN` for
every route because `git add` inside its own setup had left the marker behind.
All of that is in `target/tmp/filter_probe.sh`'s header, which is kept out of the
repository but not deleted, because the next reader will want it.

What closes it, and why refusing is the only option rather than the cautious
one, is in `docs/architecture/FINGERPRINTING.md` under "A repository is not
allowed to make Git run a program". The short form:

- **It cannot be turned off.** No Git flag disables in-tree `.gitattributes`
  (`git help --config` lists only the global `core.attributesFile`), and the
  driver name is not known until Git has read the project, so there is no fixed
  `-c` to pass.
- **Overriding it would be wrong, not just hard.** With the filter off, Git
  compares a file's raw bytes against a blob that was written *through* the
  filter and calls every such file modified. A wrong fingerprint marks stale
  evidence current, which is the failure this product exists to prevent.
- So the repository is refused, with `FingerprintError::RepositoryRunsPrograms`
  naming the settings it found. `crates/sure-core/src/fingerprint/error.rs` is
  where the message lives; it says what happened, why SURE stopped, and what to
  do.

The check is one extra invocation — `git config --list --includes -z` with
`GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL=<root>/.git/config/sure-no-global-configuration`
— placed **after** `rev-parse` (measured not to run a filter) and **before**
`status` (measured to run one). The suppressed global path is under
`.git/config`, which is a file in every repository Git makes, including linked
worktrees and submodules, so nothing can exist there and no project can add
settings to the answer about itself.

### Two claims this work made and then corrected, both by measurement

This is the part worth carrying forward, because both were wrong in the same
direction — a confident sentence that a test appeared to confirm.

1. **`--includes` is not load-bearing.** The source comment, the document and a
   test all said the flag was what made an included filter visible, reasoning
   from "includes are off as soon as a scope is named". That is true of
   `--local` and **false of this invocation**: `git config --list` with no scope
   named already follows includes, so the flag changes no answer. It was caught
   by writing a mutation for it and seeing that its removal failed *only* the
   test that pins the flags and no behavioural test. The flag is kept — stating
   the property beats inheriting it from a default — and the false claim is
   corrected in all three places.
2. **The include-path test did not test what its name said.** It was written to
   cover `--includes`, and it covers a real property instead: the check does not
   care whether a setting arrived through `include.path` or the repository's own
   file. Both the test comment and the document now say so.

The lesson is the one this repository keeps relearning: a test that passes is
not evidence that the thing it names is being tested. Only removing the thing
and watching the test fail is.

### A boundary that was measured, and is left open on purpose

**A repository can still reach a filter the *machine* defines**, by naming the
driver in `.gitattributes` without defining it. Measured with
`target/tmp/boundary_probe.sh`: on this machine both
`C:/Program Files/Git/etc/gitconfig` and `~/.gitconfig` carry
`filter.lfs.{clean,smudge,process}` and `git-lfs` is on the `PATH`, so a plain
`git status` on such a repository runs `git-lfs`.

It is left open, and the reason is the line `EXECUTION_SAFETY.md` actually
draws: the project **cannot choose the program**, only ask for one the user
already installed. What runs is not project-controlled code. That is a
defensible boundary and it is not the same thing as the defect the review named,
which let a repository choose the program.

**This needs an owner decision and no task covers it.** Closing it would mean
reading driver names out of the project's attributes (`.gitattributes`,
`$GIT_DIR/info/attributes`) and refusing when any resolves in any scope — real
work with a real cost, namely a refusal for every repository that legitimately
uses Git LFS. It is documented in full in `FINGERPRINTING.md` and raised here
rather than folded into a security fix for something else. `P2-T011` is **not**
this; that id is the intent model. No id was invented.

### Gates for `5705444`

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --
-D warnings` clean; `cargo test --workspace` green across 31 test binaries
(`fingerprint_git` went 40 → 43, `fingerprint::git` unit tests 24); `node
scripts/taskctl.mjs validate` = state OK: 166 tasks; `scripts/Preflight-Windows.ps1`
= passed. **34 mutations in `target/tmp/mutate6.py`, 34 fired** — 8 of them new
here, and the two that matter most are "the check runs after the status instead
of before it", caught only by the assertion that the marker does **not** exist,
and "the suppressed global path is one a project could create".

**On CI, at run `34843216260`, all five jobs are green.** Two things about that
run are worth more than the colour:

- The eight new refusal tests were read out of the log **by name**, on all three
  `rust` jobs. The two scope tests ran twice each on Windows, Linux and macOS.
- **Each of those tests asserts its own control** — that a raw `git status` on
  the fixture *does* create the marker — so the passing "SURE did not run it"
  assertion is meaningful on Unix and not only on Windows. This is the rare case
  where a security property is verified by execution on all three platforms
  rather than argued from one.

## Continuous integration, and why this section exists

**Every `ci` run on this branch failed until `c735a2f` — including the runs for
both accepted tasks — and no handoff said so.** Every handoff up to `P2-T002`
recorded the local gate set — fmt, clippy, the workspace suite,
`validate-bootstrap` — as "the gates", all of it green, and never opened a run.
Three of the five jobs were failing the whole time.

| Run | Commit | Result |
| --- | --- | --- |
| 34838737260 | `3fea3fa` — **the `P2-T002` acceptance commit** | failure: `rust (ubuntu-latest)`, `rust (macos-latest)`, `shellcheck-secondary` |
| 34839005174 | `58b3793` — first attempt at a fix | failure: the same three jobs |
| 34839532984 | `c735a2f` — after reading that run | **all five green.** First green run on this branch, and the first that executes any `#[cfg(unix)]` fingerprint test |
| 34840217454 | `9f13f0d` | **all five green**, including the new pipe test on both Unix jobs |
| 34843216260 | `5705444` — the filter hardening | **all five green.** The eight new refusal tests were read out of the log **by name on all three `rust` jobs**, not inferred from the job colours; `a_program_reached_through_an_included_file_is_refused_too` and `a_program_in_the_worktree_configuration_is_refused_too` ran twice each on Windows, Linux and macOS |
| 34840594638 | `91e8838` — the CI-outage record | all five green |
| 34843455162 | `9850a9a` — the filter-hardening record | all five green |
| 34845282962 | `7ce90bf` + `1cda5a0` — **the `P2-T003` acceptance** | **all five green.** Detail below, because the colour is the least of it |
| 34845645097 | `c7c0824` — the `P2-T003` record | all five green |
| 34850549120 | `0a577ca` + `68e51d8` + `1ec5bee` — **the `P2-T004` acceptance** | **failure: `rust (ubuntu-latest)` and `rust (macos-latest)`.** One test, `discover::read::tests::a_path_a_manifest_named_cannot_leave_the_project`. Detail below |
| 34851008124 | `650852e` — the fix for that | **all five green**, and the test that failed was read out of all three `rust` logs **by name** |
| 34854388756 | `e10f620` — **the `P2-T005` implementation** | **all five green.** Windows **786** / macOS **788** / Ubuntu **789** passed, 0 failed, each over 34 result lines. Detail below, because **Windows agreeing with the local run exactly is the fact worth having** |
| 34855496424 | `37a848a` — **the `P2-T005` acceptance** | **all five green, and the counts are the implementation's to the test**: Windows **786** / macOS **788** / Ubuntu **789**, 0 failed, 34 result lines = 24 parents + 10 children on each. A documentation-only commit changing no number is the useful reading — it says the record was added without touching what it records |
| 34855790124 | `cad9592` — **the record of that acceptance** | **all five green, and the same three figures a third time**: Windows **786** / macOS **788** / Ubuntu **789**, 0 failed, 34 result lines = 24 parents + 10 children on each. Read from the log rather than from the job colours: `gh run view` alone gives the conclusion, and the conclusion is the least of what a run says |
| 34858555861 | `9c931d0` — **the `P2-T006` implementation** | **all five green.** Windows **840** / macOS **842** / Ubuntu **843** passed, 0 failed, 1 ignored, each over **35** result lines = **25** parents + 10 children. The parent count went 24 → 25 because `discover_rust` is a new test binary; the child count is unchanged. **Windows agrees with the local Windows run exactly**, and all 54 new test names were read out of all three `rust` logs by name. Detail below |
| 34861419193 | `9c6e08d` — **the `P2-T006` tests commit** | **all five green.** Windows **849** / macOS **851** / Ubuntu **852** passed, 0 failed, 1 ignored, each over **35** result lines = **25** parents + 10 children. **+9 on every platform against the row above, which is the nine `pattern.rs` tests and nothing else**, and **Windows again equals the local Windows run exactly**. All nine were read out of all three logs **by name**, detail below |
| 34861757296 | `1638414` — the `P2-T006` run record | **all five green**, and **849 / 851 / 852 with 0 failed, 1 ignored** — identical to the row above. That is the reading a documentation-only commit is for: the record was added without changing what it records |
| 34862091063 | `48d1057` — **the `P2-T006` acceptance** | **all five green**, and **849 / 851 / 852 again**, unchanged. The acceptance touches only `progress/state.json` and this file, so the counts are the implementation's to the test and the three green runs together say the task's evidence survived its own recording |
| 34862387972 | `809c738` — the acceptance run's record | all five green. **This is where the chain stops**, and the stopping rule is stated rather than left implicit: every commit's run is read, but the run of the commit that *records* runs is read and reported in the session rather than enshrined in a further commit. Otherwise "read the run" never terminates. The rule was not written down before `P2-T006` and the last two tasks each ended with an unrecorded final run |
| 34864498113 | `586d3a3` — **the `P2-T007` implementation** | **all five green.** Windows **871** / macOS **873** / Ubuntu **874** passed, 0 failed, 1 ignored, each over **36** result lines = **26** parents + 10 children. The parent count went 25 → 26 because `components_graph` is a new test binary. **Windows agrees with the local Windows run exactly**, all 22 new test names were read out of all three `rust` logs **by name**, and the multisets were compared against the run before this one. Detail below |
| 34865169857 | `435181f` — the `P2-T007` run record | **all five green**, and **871 / 873 / 874 with 0 failed, 1 ignored, 26 parents** — the implementation's figures to the test, unchanged. A commit that adds only prose to this file changing no count is the reading a record commit is for |
| 34865317166 | `0907acf` — **the `P2-T007` acceptance** | **all five green**, and **871 / 873 / 874 again**, unchanged, `26` parents each. The acceptance touches only `progress/state.json` and this file, so the implementation's evidence survived its own recording. **This run is the end of the `P2-T007` chain and is reported in the session rather than committed** — see the rule stated at the top of this file |
| 34865716315 | `906bfb0` — **the `P2-T007` record commit, which edits only this file** | **failure: `rust (ubuntu-latest)`.** The other four jobs green, including `rust (macos-latest)` and `rust (windows-latest)` on **the same commit**. Two tests failed in `sure-core --test store_concurrency`. Detail below — this is the first red run since `c735a2f` and the first ever seen on a documentation-only commit |
| 34866795192 | `0f7c854` — **the fix for that run** | **all five green, including `rust (ubuntu-latest)`, the job that failed.** Windows **877** / macOS **879** / Ubuntu **880** passed, 0 failed, 1 ignored, each over **36** result lines = **26** parents + 10 children. **Windows equals the local Windows run exactly**, and the multiset comparison moved **one position on each of the three platforms** — the lib target, `401→407` on Windows and `398→404` on both Unix jobs. Detail below |
| 34869888350 | `4746c48` — **the `P2-T010` implementation** | **all five green.** Windows **907** / macOS **909** / Ubuntu **910** passed, 0 failed, 1 ignored, each over **37** result lines = **27** parents + 10 children. The parent count went 26 → 27 because `project_intent_ingest` is a new test binary. **Windows agrees with the local Windows run exactly**, and the +30 is attributed **by binary name** rather than by total. Detail below |

**The last two of the `P2-T002` runs above were missing from this table and are
added with `P2-T003`'s.** They were green and went unrecorded, which is the same
shape of gap this section exists to name — a run nobody opened is a run nobody
can describe, and "it was green" written from memory is exactly what the red
acceptance commit was written from.

### Reading run `34869888350`, `P2-T010`'s — and a delta attributed by binary name

**All five jobs green.**

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 37 | 27 | 10 | **907** | 0 | 1 |
| `rust (macos-latest)` | 37 | 27 | 10 | **909** | 0 | 1 |
| `rust (ubuntu-latest)` | 37 | 27 | 10 | **910** | 0 | 1 |

**907 is the local Windows figure exactly**, measured before the push. The raw
sum over all 37 lines is 917 / 919 / 920, which over-counts by exactly 10 for the
reason recorded above. The platform offsets are +2 and +3, unchanged.

#### The +30, attributed to four named binaries

The multisets alone say *how much* moved; they do not say *what*. The logs carry
the binary name on each `Running` line, so the delta was read out of them by
name on Windows:

| binary | before | after | delta |
|---|---|---|---|
| `sure_core` (lib) | 407 | **416** | **+9** |
| `sure` (bin) | 36 | **48** | **+12** |
| `cli_contract` | 13 | **14** | **+1** |
| `project_intent_ingest` | — | **8** | **+8** (new binary) |

**9 + 12 + 1 + 8 = 30, and every other position is identical.** That is the whole
attribution, and it matches the four places `P2-T010` put tests: nine unit tests
in `sure-core`'s `project_intent`, twelve in `sure-cli`'s `check`/`report`, one in
`cli_contract`, and the eight-test integration binary.

**The four parents the name matcher did not name are the four `Doc-tests`
targets, and reconciling that is what makes the arithmetic close.** 23 `Running`
lines plus 4 `Doc-tests` targets is 27 parents; the 23 named binaries sum to
**903**, and `Doc-tests sure_core` contributes the remaining **4** (the other
three doc-test targets have no tests). 903 + 4 = **907**. `Doc-tests sure_core`
was 4 before this commit too, so it is not part of the delta — but a reader who
subtracts 903 from 907 and finds 4 unexplained should know where it went rather
than assume the table is short.

**Two mistakes were made getting that table, and both are the kind that produce a
green that means nothing.** The first pattern was written `Running .*?deps` — but
the `Running` in a GitHub log is followed by an ANSI colour reset, not a space, so
it matched nothing, and a name matcher that matches nothing prints a
well-formed table of zeros that reads exactly like "no binary changed". The
second carried the last-seen name forward across a `Running` line it had not
matched, which labelled one binary's count with another's name and produced
`sure_domain +87` on Ubuntu and `components_graph +22` on macOS — numbers for
binaries nothing had touched. The fix is in `target/tmp/bincounts.py` (git-ignored)
and the rule is written at the top of it: **consume the name with the result line
it belongs to, and print the number of matches, because a zero-row diff and a
dead pattern are otherwise the same output.**

#### What is compared by name, what is compared by multiset, and what each can support

The per-name table is **Windows only, and deliberately**. The macOS and Ubuntu
logs interleave: cargo's output for parallel test binaries arrives with
timestamps out of order, so a name and the count beneath it are not reliably
adjacent. Every attempt to pair them there produced deltas for binaries nothing
had touched — `sure_domain +87` on Ubuntu, `components_graph +22` on macOS — and
those numbers were wrong in the way that matters, because they looked like
findings.

The multiset is sound on all three platforms, but **it can support less than it
first appears to.** Its claim is checked rather than asserted: substituting the
four Windows deltas into each platform's *before* multiset (404→413, 36→48,
13→14, and one new 8) reproduces each *after* multiset exactly, on both Unix
jobs. What it does **not** do is determine those deltas — sorting discards which
value was which, so on macOS a greedy positional alignment instead yields
`47→48, 46→47, 36→46`, a different mapping that is arithmetically consistent too.
**The multiset is consistent with the attribution; the Windows name table is what
pins it.** Recorded at this length because the tempting sentence — "the multiset
says the same thing on all three platforms" — is the one this file is supposed to
be able to refuse, and it very nearly went in.

### Reading run `34866795192`, the fix's — and a multiset comparison on all three platforms

**All five jobs green**, including `rust (ubuntu-latest)`, the one that failed.

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 36 | 26 | 10 | **877** | 0 | 1 |
| `rust (macos-latest)` | 36 | 26 | 10 | **879** | 0 | 1 |
| `rust (ubuntu-latest)` | 36 | 26 | 10 | **880** | 0 | 1 |

**877 is the local Windows figure exactly**, measured before the push and not
predicted from it. The raw sum over all 36 lines is 887 / 889 / 890, which
over-counts by exactly 10 for the reason recorded above.

The six new test names were read out of **all three** `rust` logs **by name** —
each present exactly once on each platform, `0 FAILED` lines in each log:
`a_file_another_connection_migrated_before_the_check_is_not_reported_as_foreign`,
`a_file_that_moved_to_a_newer_schema_before_the_check_is_reported_as_newer`,
`a_file_with_somebody_elses_table_is_still_refused`,
`the_check_reads_the_file_in_one_transaction`,
`a_statement_sure_ran_to_look_reports_contention_as_contention`,
`a_file_that_reported_a_version_is_not_looked_at_again`.

#### The multiset, against the run before it, on every platform

The previous run's logs were fetched rather than remembered, so the comparison is
over all three platforms and not only the one this file had recorded:

| platform | before | after | positions that moved |
|---|---|---|---|
| `rust (windows-latest)` | 871 | **877** | **1** — the lib, `401 → 407` |
| `rust (macos-latest)` | 873 | **879** | **1** — the lib, `398 → 404` |
| `rust (ubuntu-latest)` | 874 | **880** | **1** — the lib, `398 → 404` |

**Exactly one position moved on each platform, and it moved by exactly 6.** Every
other position in the 36-value multiset is identical, on all three. That is the
whole attribution: the six tests this commit adds are in the lib target and
nowhere else, and nothing else in the suite changed size.

The platform spread is likewise unchanged: macOS and Ubuntu carry 3 more lib
tests than Windows (the `#[cfg(unix)]` ones), giving +2 and +3 on the totals, and
those offsets were +2 and +3 before this commit as well.

### Reading run `34865716315` — red, on a commit that changed only this file

**This is the run that showed a defect, and the shape of the run is most of the
diagnosis.** `906bfb0` edits `progress/HANDOFF.md` and nothing else. There is no
Rust in it. It came back:

| job | conclusion |
|---|---|
| `shellcheck-secondary` | success |
| `bootstrap-validate-windows` | success |
| `rust (windows-latest)` | **success** |
| `rust (macos-latest)` | **success** |
| `rust (ubuntu-latest)` | **failure** |

Two tests failed, both in `sure-core --test store_concurrency`:

```text
thread 'child_writer' (5763) panicked at crates/sure-core/tests/store_concurrency.rs:222:49:
the store opens: Migration(Foreign { tables: ["records"] })
thread 'many_processes_opening_a_fresh_file_do_not_report_a_broken_history' (5751) panicked at crates/sure-core/tests/store_concurrency.rs:470:9:
opener 5 failed with exit status: 101
```

**The message is the finding.** `records` is the table SURE's own migration 1
creates. SURE told the user that SURE's own history file was a SQLite database
somebody else had made, and named SURE's own table as the evidence.

**Three platforms on one commit is what rules out a broken test.** A test that is
wrong is wrong everywhere; this failed on one of three. The two that passed are
not a second opinion — they are the same defect with a scheduler that did not
open the window. Every earlier run of this branch was green, so the window is
narrow, which is exactly why it survived `P2-T002` through `P2-T007`.

#### The window, and why "it is only a race" was not good enough to leave

`migrations::apply` read the version, then read the table list, with nothing
between them:

```rust
let current = version(connection)?;          // (1) what version is this?
...
if current == 0 {
    let tables = user_tables(connection)?;   // (2) does it already have tables?
```

Two processes open a file that does not exist yet and both read version 0 at (1).
One of them migrates it — the DDL and the `user_version` write are a single
transaction, so the file goes from *(0, no tables)* to *(1, `records`)* with no
bad half-state in between. The other then asks (2) and is told there is a table.
The two answers describe two different moments, and the conclusion drawn from
them — "this file is not mine" — is false.

`apply_one` already re-read the version *inside* its write transaction, and its
doc comment says why at length. The foreign-database check had no such protection
and no comment saying it needed one. **The module documented the race it had
thought about, in the function that did not have it.**

#### The fix, and the part of it no test can reach

`resolve_fresh_database(connection, seen)` now does the check, reading the
version and the tables **inside one read transaction**. Two things in it are
load-bearing and they are not the same thing:

- the version is read **again** inside, which corrects a `seen` that went stale
  before the call — the shape the failing run had;
- the two reads are **bracketed**, which stops a commit landing between them.

**Only the first is reachable from a test**, and this was measured rather than
assumed: deleting the bracketing and re-running left **all 20 tests passing**.
So there is now a test that fails when the bracketing is removed and on nothing
else — `the_check_reads_the_file_in_one_transaction`, which asks the function to
run where it must not (inside a transaction) and asserts that it refuses. It is
written from the outside because the inside needs a writer to commit in the
middle of a function, which is the race itself. That is stated in the module doc
and in the test, not left for a reader to discover.

A file that moved to a *newer* schema in the window is now `NewerSchema`. It used
to be `Foreign { tables: ["records"] }` — the same wrong answer, with the version
that actually moved left unsaid.

`MigrationError::Inspect` is new, for a statement SURE ran to *look* at the file
rather than to change it, and the mapping is unit-tested against errors SQLite
really produced (a genuine `SQLITE_BUSY` from a second `BEGIN IMMEDIATE`, and a
genuine syntax error) rather than against errors built to match the pattern.

#### How the reproduction was made deterministic, and why it had to be

The failing test is a race, and **a flaky test cannot tell a fix from a lucky
run** — that is the whole reason the fix is justified by a unit test instead. The
sequence was: extract the two reads into a named function with **today's
behaviour unchanged**, write the test, and watch it go red with the same string
CI printed —

```text
called `Result::unwrap()` on an `Err` value: Foreign { tables: ["records"] }
```

— before writing any fix. Then fix, then green. The stale version is *passed in*
rather than provoked, because it is the same input the race produces and it does
not depend on the scheduler being unkind.

The `store_concurrency` test was also run **8 times** after the fix with no
failure. That is a smoke check and is recorded as one: eight green runs of a test
that fails roughly one run in five proves very little, and it is not offered as
evidence. The deterministic test is the evidence.

### Reading run `34864498113`, `P2-T007`'s — and a comparison against the run before it

**All five jobs green**: `shellcheck-secondary`, `bootstrap-validate-windows`,
`rust (windows-latest)`, `rust (macos-latest)`, `rust (ubuntu-latest)`.

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 36 | 26 | 10 | **871** | 0 | 1 |
| `rust (macos-latest)` | 36 | 26 | 10 | **873** | 0 | 1 |
| `rust (ubuntu-latest)` | 36 | 26 | 10 | **874** | 0 | 1 |

**Windows CI printed exactly the figure the local Windows run printed, 871** —
the same agreement every run on this branch has produced, and the reason the
local gate set is worth running at all. The parent count went 25 → 26 because
`components_graph.rs` is a new test binary; the child count is unchanged at 10.
The `+2` macOS and `+3` Ubuntu deltas are the `#[cfg(unix)]` tests `P2-T002`
recorded, unchanged from the two runs before this one.

**The 22 new tests were read out of all three `rust` logs by name, not inferred
from the job colours.** Each of the 18 `components::tests::*` names and each of
the 4 `components_graph` names is `... ok` on Windows, macOS and Ubuntu, and the
count of `FAILED` lines matching those names is **0 on all three**. That check is
the one the counts cannot make.

**The name that most needed it is `the_component_graph_opens_no_file_and_starts_no_process`.**
Unlike the other three, which build a fixture under `target/tmp` and discover it,
this one reads `crates/sure-core/src/components.rs` through
`sure_testkit::repository_root()` — so it depends on the checkout layout rather
than on the fixture machinery. A path-dependent test that passes locally is
exactly the kind that can pass nowhere else, and it is `ok` on all three
platforms. Reading it by name is what makes that a fact.

**The Windows multiset came back one for one:**
`401, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×3,
0×4` — **26 values for 26 expected binaries**, with `401` appearing exactly once
and `4×3` where it was `4×2`.

**This run was compared against the run before it, which is a stronger check
than either multiset alone.** Both counts were taken the same way, over the
parent sections of each `rust` job's log, so the two are comparable:

| position | `34858555861` (`9c931d0`) | `34864498113` (`586d3a3`) | moved |
|---|---|---|---|
| lib, Windows | 374 | **401** | +27 |
| lib, macOS and Ubuntu | 371 | **398** | +27 |
| the `4` binary | `4×2` | **`4×3`** | +1 binary, 4 tests |
| **every other position** | — | — | **identical** |

**+27 is 9 + 18 and it is the two test-adding commits that sit between the two
runs, not one.** `374 → 383` was `P2-T006`'s nine `pattern.rs` tests, whose own
reading is below; `383 → 401` is this task's eighteen. 840 + 9 = 849 and
849 + 22 = 871, and 22 is 18 module tests plus the 4 in the new binary.

**The four positions where Windows and the Unix jobs differ were already
different, in the same places, in the run before this one.** Windows carries
`43` and `23` where the Unix jobs carry `47` and `25`, and macOS replaces
Windows' single `30` with a second `29`. All four are unchanged between the two
runs, so they are pre-existing platform-gated tests and **not something
`P2-T007` introduced** — which is the thing a single run's multiset cannot say,
because a difference with nothing to compare it to looks the same whether it is
old or new. This file still does **not** attribute any of the four to a named
test.

**The raw sum over all 36 lines is 881 on Windows**, over-counting by exactly 10
for the reason recorded above; macOS 883 and Ubuntu 884 raw. Every figure in this
section is the sum over the 26 parent sections.

### Reading run `34854388756`, `P2-T005`'s — and a counting method that is only sound where it was used

**All five jobs green**, and this is the first run in which **any Python
discovery test has ever executed**, on any platform.

**The names were read out of all three `rust` logs, not inferred from the
colour.** `a_project_with_no_manifest_sure_can_read_is_not_called_fully_understood`,
`discovery_runs_nothing`,
`every_file_that_marks_a_python_project_is_enough_on_its_own` and
`a_scan_that_looked_at_everything_says_so_and_one_that_did_not_says_what_it_missed`
are each `... ok` **exactly once in each of the Windows, Ubuntu and macOS logs**.
That is what makes "the Python tests ran on Unix" a fact rather than an
inference from a green job, and it is the check the counts cannot make.

| job | result lines | parents | children | passed |
|---|---|---|---|---|
| `rust (windows-latest)` | 34 | 24 | 10 | **786** |
| `rust (macos-latest)` | 34 | 24 | 10 | **788** |
| `rust (ubuntu-latest)` | 34 | 24 | 10 | **789** |

**Windows CI printed exactly the figure the local Windows run printed, 786, which
is the fact worth having.** It is a stronger statement than "green": the same
number from two independent executions on the same platform, one of them the one
that will judge every future push. The `+2` macOS and `+3` Ubuntu deltas are the
`#[cfg(unix)]` tests `P2-T002` recorded, unchanged, and 786 + 2 and 786 + 3 are
the two totals to the test.

**The acceptance commit's own run was read too — `34855496424`, on `37a848a`,
all five jobs green.** `progress/state.json` and this file are the only things it
touches, and the count came back **786 / 788 / 789 with 0 failed over 34 result
lines on every platform**, which is the implementation's figures to the test.
That is what a documentation-only commit's run is for: it says the record was
added **without changing what it records**, and it is the cheapest place to
notice that a "docs only" change was not one.

**A method lesson, learned here by getting it wrong twice.** Attributing a
`test result:` line to a target **by proximity in a CI log is invalid**. Cargo
writes `Running <target>` to stderr and the test harness writes `test result:` to
stdout, and the runner merges the two by arrival — so on CI a target's result
lines appear **before** the `Running` line that produced them. Read that way, a
macOS log showed `discover_node` at **9 passed** for a 33-test binary, and
`discover_python` at **33** and again at **31** in the same log. None of those
three numbers is a fact about the binary; they are facts about interleaving.
There is one child signature that survives this, because `store_concurrency`'s
children print a line no other target can print — `0 ignored; 0 measured; 6
filtered out` — so the ten children can be identified and subtracted with
confidence. **The figures above are counts over a whole job's step, minus those
ten.** Per-target counts are quoted in this file **only from the local run**,
where both streams reach one pipe in write order and the attribution is sound.

### Reading run `34861419193`, the `P2-T006` tests commit's

**All five jobs green**, and the count is the one a tests-only commit has to come
back with: **849 / 851 / 852 passed, 0 failed, 1 ignored, over 35 result lines =
25 parents + 10 children on every platform.** Against the row above that is
**+9 on each of the three**, and +9 is exactly the number of tests the commit
added — so the commit is what it says it is and nothing else moved. **Windows
849 equals the local Windows run exactly**, which is the same agreement the
implementation commit produced and the reason the local gate set is worth running
at all.

**All nine `pattern.rs` tests were read out of all three `rust` logs by name, and
`... ok` on each.** That mattered more here than for a normal test: one of the
nine branches on the platform, and its **Unix half had never executed anywhere**
before this run. `an_absolute_pattern_is_refused_where_this_platform_says_it_is_absolute`
asserts that `C:\Windows` is refused on Windows and is **one ordinary file name**
on Unix, where a backslash is legal in a name and there is no drive letter to
read. Windows takes the first branch; macOS and Ubuntu took the second for the
first time, and both passed.

**The Windows multiset is the per-target check, and it came back one for one:**
`383, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×2,
0×4` — **25 values for 25 expected binaries.** The only two that moved against
the previous run are `374 → 383` (the lib, +9) and the unchanged `24`
(`discover_rust`), and `383` appears exactly once, which is what makes the
attribution usable rather than merely suggestive. The same caveat as before
applies: two binaries could in principle share a count, so this is a check that
would catch a wrong figure and is not a proof.

**The raw sum over all 35 lines is 859 on Windows, over-counting by exactly 10**
for the reason recorded above; macOS 861 and Ubuntu 862 raw. Every figure in this
section is the sum over the 25 parent sections.

### Reading run `34858555861`, `P2-T006`'s — and the arithmetic that bit twice

**All five jobs green**, and the first run in which any **Rust** discovery test
has executed anywhere.

| job | result lines | parents | children | passed | failed | ignored |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 35 | 25 | 10 | **840** | 0 | 1 |
| `rust (macos-latest)` | 35 | 25 | 10 | **842** | 0 | 1 |
| `rust (ubuntu-latest)` | 35 | 25 | 10 | **843** | 0 | 1 |

**Windows CI printed exactly the figure the local Windows run printed, 840**, the
same agreement `P2-T005` had at 786. The section count went 34 → 35 because
`discover_rust` is a new test binary, so the parents went 24 → 25; the child count
did not move. The `+2` and `+3` deltas are the `#[cfg(unix)]` tests `P2-T002`
recorded, unchanged from the previous two runs.

**A trap in the counting, worth writing down because it was stepped in.** The raw
sum over all 35 result lines is **850** on Windows — and it is wrong by exactly
**10**. `store_concurrency`'s ten children each print
`1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out`, and the parent
section's own line already counts those same tests, so summing every line counts
them twice. **The figure to report is the sum over the 25 parent sections**, which
is 840. The first pass of this run's count printed 850 and the number looked
plausible; it was caught only because it disagreed with the local run. That is the
argument for having both numbers: a figure read one way and a figure computed
another way, and the disagreement is the signal.

**All 54 new test names were read out of all three `rust` logs by name** — the 30
`#[test]`s in `rust.rs` and the 24 in `discover_rust.rs`, each `... ok` in each of
the Windows, macOS and Ubuntu logs, 54/54 on every platform. The check is
mechanical rather than by eye: parse the two files for functions carrying a
`#[test]` attribute, then require `test <name> ... ok` to appear in the job's
lines. A first attempt at this reported 55 names "missing" on every platform and
all of them were artifacts of the extraction, not facts about the run: the lib
target qualifies its test names with the module path
(`test discover::rust::tests::<name> ... ok`), and the integration file's helper
`fn listing` is not a test at all. **A name check that has not been checked
against a name that certainly ran is not evidence**; the two errors here were
opposite in direction from the count error above, and both were found by the
number being implausible rather than by being read.

Per-target attribution is still unavailable for the reason recorded above: a
24-test result line appears **exactly once** in each of the three logs, which is
consistent with `discover_rust`'s 24 tests, and consistency is all it is.

### Reading run `34851008124`, the fix for the red acceptance

**All five jobs green, and the test that failed was read out of all three `rust`
logs by name rather than taken from the colour of the job.** The string
`a_path_a_manifest_named_cannot_leave_the_project ... ok` appears **exactly once
in each** of the Windows, Ubuntu and macOS logs — the same test that failed on
two of them one run earlier. Zero `FAILED`, zero `error: test failed`, in all
five logs. The only occurrences of the word `panicked` anywhere in the log are
inside the name of a test that passed
(`scan::ignore::tests::a_path_that_climbs_out_of_itself_is_answered_rather_than_panicked_over`),
which is worth stating plainly because a grep for the word returns three hits.

**The counts, and a subtraction that has to be written down because the number a
raw sum gives is not the number this file quotes.** Summing every `test result:`
line in a job gives **736 / 739 / 738** (Windows / Ubuntu / macOS). Every figure
this file has quoted for a workspace run is **726 / 729 / 728** — exactly ten
less, on each platform. The ten are `store_concurrency`'s child processes, four
writers and six openers, each printing

```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.79s
```

Their lines *are* in the CI log, so a sum that does not know about them counts
them. Both figures are arithmetic on the same log, and only the second is the
workspace's test count.

**`33 result lines` decomposes the same way, and reconciles: 23 parent sections
plus those 10 children.** The 23 are the 19 test binaries and the 4 doc-test
targets, in that order in the log. Taken section by section for Windows:

| | count |
|---|---|
| test binaries | 19 |
| doc-test targets | 4 (`sure_core` 4 passed, `sure_domain`, `sure_protocol`, `sure_testkit` 0 each) |
| parent sections | **23** |
| `store_concurrency` children | 10 |
| result lines | **33** |
| parents' passed, summed | **726** |

The four zero-or-small `Doc-tests` targets matter beyond bookkeeping: a
zero-test section still prints a `test result:` line and prints **no `test ...`
lines at all**, which is the one-line shape a short capture loses. See "the
32-vs-33 paragraph" above — this run is the evidence for it, and the lost line
is a property of capture rather than of the suite.

**Per platform, unchanged where it should be and different where it must be:**

| job | result lines | parents | children | parent passed | `sure-core` lib | `discover_node.rs` |
|---|---|---|---|---|---|---|
| `rust (windows-latest)` | 33 | 23 | 10 | **726** | 315 | 33 |
| `rust (ubuntu-latest)` | 33 | 23 | 10 | **729** | 312 | 33 |
| `rust (macos-latest)` | 33 | 23 | 10 | **728** | 312 | 33 |

The `sure-core` lib is 315 on Windows and 312 on both Unix jobs, and the
`+3`/`+2` spread against the Unix totals is the same one `P2-T003` recorded. The
new integration binary is **33 on all three**, which is the number `--list`
gave locally.

**What this run does and does not settle.** It settles that the fix compiles and
passes **on Unix**, which is the thing that could not be checked here and the
reason the fix was pushed at all. It does not settle the `#[cfg(unix)]` arm of
`link_to_directory`: that test is in `discover_node.rs`, which ran 33 tests on
each platform, and its Unix arm running green there is evidence about that arm
only to the extent the test names can be matched up — which they have not been,
test by test, and this file should not imply otherwise. What was verified by
name is the one test whose failure started this.

### Reading run `34850549120`, `P2-T004`'s — and a claim in this file that it falsified

**Red on both Unix jobs, green on all three Windows-side jobs.** Windows
`rust (windows-latest)` passed, `bootstrap-validate-windows` passed, so a
Windows-only gate set would have shown this acceptance as clean. That is the
pattern this section exists for.

One test, read out of the log by name and by message rather than inferred from
the colour:

```
test discover::read::tests::a_path_a_manifest_named_cannot_leave_the_project ... FAILED
panicked at crates/sure-core/src/discover/read.rs:571:13:
"C:\\Windows" was accepted as a path inside the project
```

on **both** `rust (ubuntu-latest)` and `rust (macos-latest)`.

**The code was right and my test was wrong.** A Windows-spelled absolute path is
not absolute on Unix: with no `/` in it, `C:\Windows` is a single
`Component::Normal` — one relative name, and a legal name for a file inside the
project. `contained_relative` accepting it is correct on that platform and
harmless, because the caller then looks for a directory by that name and finds
nothing. The test had listed `r"C:\Windows"` and `r"\\server\share"` among the
strings that must be refused, which is true only where those spellings are
absolute.

**The claim this falsified is worth recording, because I wrote it in this file
one commit earlier.** `P2-T004`'s entry said:

> This task adds no platform-gated test, so nothing in it is invisible to a
> Windows run the way the `#[cfg(unix)]` link tests are.

**That is false, and it was the reason the failure was a surprise.** The task has
three platform-dependent surfaces, not none: a `#[cfg(windows)]`/`#[cfg(unix)]`
pair for `link_to_directory` (a **directory** link via `mklink /J` on Windows and
`std::os::unix::fs::symlink` on Unix), whose Unix arm has never run here; this
assertion; and the general class the sentence missed — an assertion about
*platform behaviour* is invisible to a Windows run even when no `#[cfg]`
attribute appears anywhere near it. The lesson is not "list the gated tests"; it
is that **"this change is platform-independent" is itself a claim needing
evidence, and the only evidence is a run on the other platform.**

**The fix, and what it does and does not claim.** The Windows spellings are now
asserted per platform. On Windows they must be refused, as before. On Unix the
premise is asserted first — `!Path::new(name).is_absolute()`, so the test says
*why* accepting it is right instead of guessing — and then the function's
**contract** rather than a spelling: whatever comes back must be neither absolute
nor rooted.

That last choice is deliberate: the first draft of the fix asserted the exact
`Some(PathBuf::from(name))`, which is a guess about Rust's normalization made by
the same reasoning that produced the bug. Asserting the contract —
*the answer cannot leave the project* — is the statement that actually has to
hold, and it is the one a future refactor should be held to.

**Not verified on this machine, and this time it is written down before the push
rather than after.** `cargo clippy --target x86_64-unknown-linux-gnu` cannot run
here for `--workspace --all-targets`: `rusqlite`'s bundled SQLite needs
`x86_64-linux-gnu-gcc`, which is not installed, so the cross-target check the
environment notes describe is unavailable for anything that links the store. The
`#[cfg(unix)]` arm of this assertion therefore **has never been compiled or run
anywhere**, exactly like `link_to_directory`'s. The macOS and Ubuntu jobs are its
first execution, and the run has to be read rather than assumed.

### Reading run `34845282962`, `P2-T003`'s

Not the colour — the log. Four things were taken out of it:

- **The totals differ by platform, and two independent readings agree.** Windows
  **668** passed, Linux **671**, macOS **670**, with 0 failed and 1 ignored on
  each. Every job's log has 32 `test result:` lines, which is 18 binaries + 4
  doc-test targets + `store_concurrency`'s **10 children**; the children account
  for exactly 10 of the "passed" sum, and 678 − 10 = 668 is the figure the local
  Windows run printed as well. That agreement is what makes the Linux and macOS
  figures trustworthy rather than merely plausible.
- **The two new `#[cfg(unix)]` tests ran on both Unix jobs**, read by name:
  `a_link_is_recorded_by_where_it_points_and_never_read_through` and
  `a_pipe_in_the_project_is_refused_rather_than_opened` are `ok` in the Ubuntu
  and the macOS log, and absent from the Windows one. The second exists for the
  same anti-hang reason the Git kind's does, and it has now run somewhere.
- **The platform set difference is measured, and it is what was predicted.**
  Windows-only names: **8**, unchanged. Linux-only: **11** — the nine of
  `9f13f0d` plus the two above. Net **+3 on Linux**, and 668 + 3 = 671, which is
  the total the log printed. macOS against Windows is **12**, net **+2**, and
  668 + 2 = 670. The gate-set section predicted "+3"; it was right, and it was
  checked rather than left standing as a prediction.
- **Nothing was silently skipped on any platform.** Every test function in
  `fingerprint_content.rs` and `fingerprint_git.rs` was looked for **by name** in
  each of the three jobs' lists, and the only ones missing are that platform's
  `#[cfg]` gates — `23 + 2` on Unix and `43 + 4` on Windows. That is the check
  that would catch a target quietly running nothing, and it is the reason to
  count names rather than to read the summary lines.

`target/tmp/diff_names.py` (git-ignored, ad hoc) is the extractor. It reads
doctest lines as source paths, so its retained-name list carries a few junk
tokens; those are identical on every platform and cancel in the set difference,
but anyone reusing it should filter on the path separators rather than trust its
counts. The `23 + 2` and `43 + 4` above were counted from the source, not from
it.

The acceptance commit's own run is red. That is the fact this section exists for:
`P2-T002` was marked accepted, and `progress/state.json` says so, on a commit
whose CI failed — and the acceptance was sound only because none of the three
failures happened to touch the behaviour being accepted.

**The first fix was a guess and did not work; reading the log is what fixed it.**
`58b3793` was written from local reasoning about what could be wrong, touched the
three files that reasoning named, and pushed. The same three jobs failed again.
Reading `--log-failed` then named the causes exactly:

| Failure | The log's words | Why the local gate set cannot see it |
| --- | --- | --- |
| `preflight.sh` — the fix for SC1128 *introduced* SC1072/SC1073 | `Couldn't parse this shellcheck directive` | nothing on Windows runs `preflight.sh`; the shellcheck job is its only caller, and the file is one of the files its own last line checks. The new error came from a comment whose **first word** was the linter's own name |
| `scan_project.rs:272` on Linux | `assertion left == right failed` in `a_name_that_is_not_valid_unicode_...` | the assertion expected one U+FFFD; Unix decodes the bytes to three, by the maximal-subpart rule. `58b3793` had fixed this test's *gating* and left its *assertion* |
| `paths/compare.rs:440` on macOS | `the_platform_rule_is_applied_by_the_default_entry_point` | that test was under a bare `#[cfg(unix)]` asserting **Linux's** case rule. `unix` includes macOS, whose default volume is case-insensitive |

Two of the three are the same shape: **code gated to a set of platforms that is
not the set it is correct on.** `#[cfg(unix)]` is not "where this is used" and
not "where this is true"; it is a list of platforms, and the list that compiles a
helper and the list that can run it have to be the same list. The third was a
test asserting one platform's answer on another — the same mistake as the
second, arrived at independently.

A fourth defect was found in the same reading and is unrelated to platforms: a
path Git reports could **climb out of the project** (see "What the hardening
added" above). It is recorded here because the local gate set could not see it
either — nothing in it feeds a hostile repository to the fingerprint.

**The local Windows gate set structurally cannot substitute for CI**, and this
was established by trying rather than by reasoning: `cargo check -p sure-core
--target x86_64-unknown-linux-gnu` fails in `cc-rs` with *failed to find tool
"x86_64-linux-gnu-gcc"*, and this machine has no clang, gcc or zig. A stub `cc`
emitting empty objects was considered and **rejected**: it could make a real
failure look green, which is the one thing worse than a red build.

**What can be done locally about platform-gated code** — and was, for `9f13f0d` —
is to compile the *constructs* rather than the code: a throwaway file on the host
that uses the same expression shapes under `-D warnings`. That catches a
type error in a `#[cfg(unix)]` body; it does not catch a wrong *expectation*, and
nothing local can. Only a run on the platform can.

`docs/development/GITHUB_WORKFLOW.md` now carries the rule this produced —
**a push is not finished until its run has been read** — with two reading
consequences: `cargo test` in CI runs without `--no-fail-fast`, so a job's log
stops at the first failing target, and a green `windows-latest` job says nothing
whatsoever about the other two.

## Gate set, as run on the fix for run `34865716315`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **877 passed, 0 failed, 1 ignored, across 36 result lines = 26 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| mutation check, 5 mutations over the new branches | **5 of 5 caught**, baseline green first; one printed `SKIP` on the first pass because its anchor appeared twice, and was re-anchored rather than counted |

**877 is 871 + 6, and the six are the tests this fix adds and nothing else.**
Measured, not predicted: the count was taken from the parent sections before the
fix was written and again after. The raw sum over all 36 lines is **887**, which
over-counts by exactly 10 for the reason recorded above.

The fix also ran `sure-core --test store_concurrency` **8 times** with no failure.
Recorded here so the number is not mistaken for evidence: a test that fails about
one run in five passing eight times is a smoke check, and the deterministic unit
test is what justifies the fix.

## Gate set, as run on `P2-T010`'s implementation commit

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **907 passed, 0 failed, 1 ignored, across 37 result lines = 27 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate12.py` | **21 of 21 observable mutations caught, 2 declared unobservable and missed as declared, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**907 is 877 + 30, and 877 is the Windows figure this file already recorded for
`3be88f1`** — the commit this work starts from — so the arithmetic does not have
to cross the migration-race fix in the middle. The thirty are: twelve in
`sure-cli`'s binary (nine in `check.rs`, two in `report.rs`, one in
`commands.rs`), one in `cli_contract.rs`, nine in `sure-core`'s lib
(`project_intent.rs`), and eight in the new `project_intent_ingest` binary.
Counting `test result:` lines alone would have given 917, which over-counts by
exactly 10 for the reason recorded above.

The result-line count moved from 36 to 37 because `project_intent_ingest.rs` is a
new test binary, so the parents went from 26 to 27. The per-binary multiset — the
only sound way to attribute a count to a target — is now
`416, 87, 48, 46, 43, 33, 31, 30, 29, 24, 23, 15, 14, 12, 9, 8×2, 7, 6×2, 4×3, 0×4`:
27 values for 27 expected binaries. Against `586d3a3`'s
`401, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×3, 0×4`,
three positions moved and one is new: the CLI binary 36 → 48, `cli_contract` 13 →
14, a new `8` for the new integration test, and the lib 401 → 416 — that last
being +6 from the migration-race fix that landed between the two runs and +9 from
this one.

**That local attribution was then confirmed by CI, binary by binary**, which is
worth recording because it is the first time the two methods have been compared
directly rather than used one at a time. Locally the multiset said the CLI binary
gained 12, the lib 9, `cli_contract` 1, and a new binary 8; CI read the same four
numbers out of the `Running` lines by name, against a different baseline run.
The local figures for the CLI binary and the lib also agree with the file-level
count — nine in `check.rs` plus two in `report.rs` plus one in `commands.rs` is
twelve — so the same delta is now reachable three ways, and "Reading run
`34869888350`" records what each of them can and cannot support.

## Gate set, as run on the `P2-T007` commit `586d3a3`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **871 passed, 0 failed, 1 ignored, across 36 result lines = 26 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate11.py` | **20 of 20 observable mutations caught, 0 declared unobservable, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**871 is 849 + 22 and 401 is 383 + 18, and the two numbers are different on
purpose.** The lib gains 18: sixteen module tests plus the two added to close the
mutation holes. The workspace gains 22: those 18, plus the four in the new
`components_graph` integration test binary. Counting `test result:` lines alone
would have given 881, which over-counts by exactly 10 for the reason recorded
above — the figure to report is the sum over the 26 parent sections.

The result-line count itself moved from 35 to 36 because `components_graph.rs` is
a new test binary, so the parents went from 25 to 26. The per-binary multiset —
the only sound way to attribute a count to a target — is now
`401, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×3, 0×4`:
26 values for 26 expected binaries, with `401` appearing exactly once and `4×3`
where it was `4×2` before, which is the new binary and nothing else.

## Gate set, as run on the `P2-T006` tests commit `9c6e08d`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **849 passed, 0 failed, 1 ignored, across 35 result lines = 25 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate10.py` | **32 of 32 observable mutations caught, 2 declared unobservable, 0 SKIP, 0 BUILD**, and `BASELINE IS NOT GREEN` did not fire |

**849 is 840 + 9 and 383 is 374 + 9, both measured rather than predicted** — the
nine are the `pattern.rs` tests and nothing else, which is the arithmetic a
commit that claims to add only tests has to come back with. The raw sum over all
35 lines is **859**, which over-counts by exactly 10 for the reason recorded
above; the figure to report is the sum over the 25 parent sections.

**The important line is the last one, and it is not the mutation count.** The
harness now runs the suite unmutated first and refuses to report anything if it
is not green. That check exists because its absence produced a run in which every
mutation was `CAUGHT` and the verdicts meant nothing — the details are under
"What the second `P2-T006` commit added". A gate set is only as good as the
question *what would this have looked like if it were wrong*, and this is the
second time on this branch that the answer was "exactly the same".

## Gate set, as run at `9c931d0` (`P2-T006`)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **840 passed, 0 failed, 1 ignored, across 35 result lines = 25 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `python target/tmp/mutate10.py` | **26 of 26 observable mutations caught, 1 declared unobservable, 0 SKIP, 0 BUILD** |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **374 lib** + 9 `config_loading` + 33 `discover_node` + 31
`discover_python` + **24 `discover_rust`** + 6 `doctor` + 23 `fingerprint_content`
+ 43 `fingerprint_git` + 30 `scan_project` + 6 `store_concurrency` + 4
`store_packaging`; `sure-domain` 87 lib + 29 `wire_contract`; `sure-protocol` 46
lib + 12 `conformance` + 15 `round_trip`; `sure-testkit` 0 lib + 7
`integration_thinness` + 8 `repository_shape`; `Doc-tests sure_core` **4**. That
is 49 + 583 + 116 + 73 + 15 + 4 = 840.

**The arithmetic against 786 at `e10f620` is +54, and both parts are counted
rather than inferred from the total**: **+30 `#[test]`** in `rust.rs`, which is
the whole of the lib movement 344 → 374, and **+24** for the new `discover_rust`
target, which is the **21st test binary** and takes the parent sections from 24 to
25. 30 + 24 = 54, and `pattern.rs`, `mod.rs`, `node.rs` and `discover_node.rs`
added **zero** — all four are in the diff and a file being touched is not a test
being added.

**Those per-binary figures were confirmed against the CI log rather than left as a
derivation**, and the way it was done is worth keeping because it is the one
per-target attribution that *is* sound on CI. Proximity is invalid (see above),
but the **multiset** of result counts is not: take every `test result: ok. N
passed` line in a job, and compare the multiset of `N` against the per-binary
counts expected from the local run. On Windows the multiset came back as
`374, 87, 46, 43, 36, 33, 31, 30, 29, 24, 23, 15, 13, 12, 9, 8, 7, 6×2, 4×2, 0×4`
— **25 values, matching the 25 expected binaries one for one**, with `374` and
`24` each appearing exactly once, which is what a unique attribution needs. It is
still not proof, because two binaries could in principle share a count; it is a
consistency check, and it is a far stronger one than a colour.

## Gate set, as run at `e10f620` (`P2-T005`)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **786 passed, 0 failed, 1 ignored, across 34 result lines = 24 parent sections + 10 children** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **344 lib** + 9 `config_loading` + 33 `discover_node` + **31
`discover_python`** + 6 `doctor` + 23 `fingerprint_content` + 43
`fingerprint_git` + 30 `scan_project` + 6 `store_concurrency` + 4
`store_packaging`; `sure-domain` 87 lib + 29 `wire_contract`; `sure-protocol` 46
lib + 12 `conformance` + 15 `round_trip`; `sure-testkit` 0 lib + 7
`integration_thinness` + 8 `repository_shape`; `Doc-tests sure_core` **4**. That
is 49 + 529 + 116 + 73 + 15 + 4 = 786.

**The arithmetic against 726 at `68e51d8` is +60 and every part is measured, not
inferred from the total.** `git diff 68e51d8..e10f620` gives **+25 `#[test]`** in
`python.rs` and **+4** in `read.rs` — the four `toml`-conversion tests — which is
the whole of `sure-core`'s lib movement, **315 → 344**; and **+31** for the new
`discover_python` target, which is also the **20th test binary** and takes the
parent sections from 23 to 24. 25 + 4 + 31 = 60. **`scan/ignore.rs` added zero
`#[test]`** — its new rows are `assert_eq!`s inside the existing vendored-set
tests — and `mod.rs` and `discover_node.rs` added none either, which is worth
stating because all three files are in the diff and a file being touched is not
a test being added.

## Gate set, as run at `68e51d8` (`P2-T004`) and `0a577ca`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **726 passed, 0 failed, 1 ignored, across 19 test binaries and 4 doc-test targets** (all 4 ran a test) |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **315 lib** + 9 `config_loading` + **33 `discover_node`** + 6
`doctor` + 23 `fingerprint_content` + 43 `fingerprint_git` + 30
`scan_project` + 6 `store_concurrency` + 4 `store_packaging`; `sure-domain` 87
lib + 29 `wire_contract`; `sure-protocol` 46 lib + 12 `conformance` + 15
`round_trip`; `sure-testkit` 0 lib + 7 `integration_thinness` + 8
`repository_shape`; `Doc-tests sure_core` **4**. That is 49 + 469 + 116 + 73 +
15 + 4 = 726.

**The arithmetic against 668 at `7ce90bf` is +58, and every part is measured
rather than inferred from the total:** **+24** in `sure-core`'s lib target (291
→ 315), all of them `discover::`, taken from `cargo test -p sure-core --lib --
--list | grep -c '^discover::'`; **+33** for the new `discover_node` target,
which is also the **19th test binary**; and **+1** doc-test, `discover::discover`.
24 + 33 + 1 = 58.

**The 668 in that comparison is right, and three earlier figures in this
session's working notes were wrong.** They are recorded because the way they were
wrong is the useful part: `669` came from subtracting the two new *sources* from
the total and forgetting that one of the additions is a **doc test**, which
belongs to no new file; `723` was a real measurement of this same tree taken when
`discover_node` held 30 tests rather than 33, so it is a measurement of a state
that no longer exists and not a contradiction; and `725` came from believing
"two tests were added" when **three** were. The rule under "Counting `#[test]`
attributes" applies to deltas as much as to absolutes: take the number from a
run, and take the per-target figure from `--list` rather than from a file's
contents.

## Gate set, as run at `7ce90bf` (`P2-T003`)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **668 passed, 0 failed, 1 ignored, across 18 test binaries and 4 doc-test targets** (3 of which ran a test) |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |
| `pwsh scripts/Preflight-Windows.ps1` | `SURE Windows preflight passed.` |

Per binary on this platform: `sure-cli` 36 **bin** + 13 `cli_contract`;
`sure-core` **291 lib** + 9 `config_loading` + 6 `doctor` + **23
`fingerprint_content`** + **43 `fingerprint_git`** + 30 `scan_project` + 6
`store_concurrency` + 4 `store_packaging`; `sure-domain` 87 lib + 29
`wire_contract`; `sure-protocol` 46 lib + 12 `conformance` + 15 `round_trip`;
`sure-testkit` 0 lib + 7 `integration_thinness` + 8 `repository_shape`;
`Doc-tests sure_core` **3**. That is 665 + 3 = 668.

**The arithmetic against the last recorded total, 628 at `9f13f0d`, is +40, and
the parts are each measured rather than inferred from the total:** **+23** for
the new `fingerprint_content` target; **+7** in `sure-core`'s lib target (284 →
291), which `git grep -c '#\[test\]'` accounts for **by file** — `git/mod.rs`
3 → 8, `choose.rs` absent → 1, `content.rs` absent → 1, and no other file in
`crates/sure-core/src` changed its count; **+8** in `fingerprint_git` (35 → 43 on
this platform, and `5705444`'s own entry already records 40 → 43 of that, from
the filter refusals); **+2** doc-tests, which are the `no_run` examples on
`content_fingerprint` and `project_fingerprint`. 23 + 7 + 8 + 2 = 40.

**The aggregate attribute count is not the test count and is not used here.**
`crates/sure-core/src` holds 297 `#[test]` lines against 291 lib tests on this
platform, and the difference is the tests gated to the other one — three by an
attribute on the function, plus the `#[cfg(unix)]` test modules in
`paths/compare.rs`. That is a second reason the note under "Count the parent
lines" applies; the per-file *delta* is exact and the absolute figure is not.

**The two platforms do not run the same tests, and the count now differs by
more than it did.** Both new `#[cfg(unix)]` tests are in `fingerprint_content`, so
that target is 23 here and **25 on Unix**, and `fingerprint_git` is 43 here and
**47 on Unix**. The set-difference table below was measured at `9f13f0d` and is
therefore **stale** — the Linux-only set gains those two names, and the
prediction written here was **net +3 on Linux, 668 + 3 = 671**. Nothing on this
machine could run them, and `target/tmp/diff_test_names.py` reads the names out of
a run's logs. **Run `34845282962` measured it and the prediction held**: 671 on
Linux, 670 on macOS, and the sets are 8 Windows-only and 11 Linux-only. See
"Reading run `34845282962`" below for the arithmetic.

## Gate set, as run at `P2-T002`

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features --no-fail-fast` | **624 passed, 0 failed, 1 ignored, across 17 test binaries and 4 doc-test targets** |
| `node scripts/taskctl.mjs validate` | `state OK: 166 tasks` |

Per binary: `sure-cli` 36 **bin** + 13 `cli_contract`; `sure-core` **281 lib** + 9
`config_loading` + 6 `doctor` + 30 `scan_project` + 6 `store_concurrency` + 4
`store_packaging` + **34 `fingerprint_git`**; `sure-domain` 87 lib + 29
`wire_contract`; `sure-protocol` 46 lib + 12 `conformance` + 15 `round_trip`;
`sure-testkit` 0 lib + 7 `integration_thinness` + 8 `repository_shape`;
`Doc-tests sure_core` 1.

The arithmetic: 553 at `P2-T001`, **624 here (+71)** = 37 in `sure-core`'s lib
target (244 → 281: 8 digest + 5 error + 16 status + 3 `fingerprint/mod.rs` + 5
in `scan/ignore.rs` for `left_out`) plus the new `fingerprint_git` integration
target's **34 on Windows, 37 on Unix** (the three `#[cfg(unix)]` bodies are
compiled only there, so the CI number will be three higher and is not a
discrepancy). The **17** binaries are 16 plus `fingerprint_git`; the previous
entry's "16" was right for its commit.

The single `ignored` is not new and not a gap being hidden: it is
`store_concurrency.rs:167`, `#[ignore = "spawned by the parent tests, not run on
its own"]`, and it has read that way since `P1-T005`.

### Gate set, as run at `9f13f0d`

The same commands, after the hardening described under "What `P2-T002` added":
**628 passed, 0 failed, 1 ignored, across the same 17 test binaries and 4
doc-test targets** — 624, plus **three** containment tests added to `git/mod.rs`
by `c735a2f` (`a_path_that_climbs_is_refused_with_or_without_a_prefix`,
`a_path_inside_the_prefix_is_still_accepted`,
`the_prefix_comes_off_as_path_components_and_not_as_text`, all in `sure-core`'s
**lib** target), plus one for the Git safety arguments in the `fingerprint_git`
integration target. 624 + 3 + 1 = 628, which is what the run printed.

`fingerprint_git` is therefore 35 on Windows and **39 on Unix** — the same 35 plus
the four `#[cfg(unix)]` bodies, three of which predate the `P2-T002` session.

**The two platforms do not run the same number of tests, and the difference is
now enumerated rather than waved at.** Linux reports **629**, one more than
Windows, and the arithmetic is a set difference over test *names* — by name and
not by target, because attributing a result line to its binary depends on the
`Running …` markers landing in order and in a CI log they do not:

| | Count |
| --- | --- |
| names that exist only on Windows | **8** — the seven in `paths::compare::tests::windows`, plus `fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_refused_on_windows` |
| names that exist only on Linux | **9** — the four `#[cfg(unix)]` tests in `fingerprint_git.rs`, the three in `paths::compare::tests::unix`, `doctor::tests::a_path_through_a_file_cannot_be_looked_at`, and `fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_the_path_on_unix` |
| net | **+1 on Linux**, which is 628 + 1 = 629 |

Every one of the seventeen is `ok` on the platform where it ran, and no test on
either platform is anything but `ok` or `ignored`. The macOS job is a third set
again: it runs `unix::the_default_entry_point_folds_case_on_a_case_insensitive_platform`
where Linux and Windows run neither that nor `…_folds_nothing_…`, so **a green
macOS job and a green Linux job are not the same evidence** even for the same
file.

`target/tmp/diff_test_names.py` (git-ignored) is what produced this table and is
worth reusing after any push that touches platform-gated code.

The `21 targets` figure some tooling prints is **17 binaries + 4 doc-test
targets**; the two are the same number arrived at two ways, not two
measurements.

## What `P2-T002` added

`crates/sure-core/src/fingerprint/` — **the answer to "is this still the same
project?"**, and the one thing every evidence record is stamped with
(`docs/architecture/EVIDENCE_MODEL.md`: evidence whose fingerprint differs from
the current one is stale). Three files:

- `digest.rs` — `Digest`, a length-prefixed incremental SHA-256. **The domain tag
  is the first field written** (`sure.git-fingerprint.v1`), so a digest of one
  kind can never equal a digest of another; `field()` writes the byte length
  before the bytes, which is what stops `("ab", "c")` and `("a", "bc")` from
  being one digest. `optional()` writes a presence byte, not an empty string, so
  "absent" and "empty" are two states — the same distinction the change list
  needs. `hash_file` reads `limit + 1` bytes and **errors** above the limit
  rather than hashing a prefix: a hash of the first megabyte of a two-gigabyte
  file is a fingerprint of something nobody has.
- `git/mod.rs` — `Git::fingerprint(root)`. It asks Git for `status` and `HEAD`
  and then **reads the files themselves**; the digest is over file contents, not
  over Git's diff (the reasoning is in `FINGERPRINTING.md`, and it is what makes
  the fingerprint survive a `git commit --amend`, a rebase and a stash).
  `--no-renames` because a rename reported as rename-or-delete must not depend on
  Git's similarity threshold; `--untracked-files=all` because a nested checkout
  is otherwise one line with no contents; `-- .` plus `--show-prefix` because
  `--relative` **silently prints nothing at all** on Git 2.55.0 for a repo with
  changes and exit status 0; the prefix is stripped with `Path::strip_prefix`,
  not string comparison, so a project in `app/` is not confused by a sibling
  `app-old/`. Excluded directories are skipped **before** they are read, and the
  walk's losses are an error (`IncompleteTree`), never a hash of a partial tree.
- `mod.rs` — `FingerprintOptions`, `FingerprintKind`, `FingerprintError` (11
  variants), and the `git_fingerprint` entry point. The record type itself is
  `sure_domain::vocabulary::ProjectFingerprint` — `id`, `kind`, `digest`,
  `git: Option<GitState>` — with `content(..)` and `git(..)` as **constructors,
  not fields**; `mod.rs` re-exports rather than redefines it, so the wire type
  the protocol and the store already carry is the one being produced.
  **`matches` compares `kind` and `digest` and never `id`.** There is no partial
  fingerprint and no "unknown" state: a fingerprint either exists or an error
  says why not — an `Option<ProjectFingerprint>` whose `None` meant "could not
  tell" would be compared as "not equal" by every caller and quietly invalidate
  all evidence.
- `crates/sure-core/tests/fingerprint_git.rs` — 34 tests (37 on Unix) against
  real repositories in the same scratch-directory pattern as `scan_project.rs`,
  with `FIXTURE_LIMIT = 5_000`. Only the tests that need a specific Git version's
  *text* pin `GIT_AUTHOR_DATE`; the rest assert relationships between two
  fingerprints rather than literal digests, so a Git upgrade cannot turn them red
  for a reason that is not a bug.
- `docs/architecture/FINGERPRINTING.md` (new) — the rule (**a file is part of the
  fingerprint if and only if a check could read it**), the two-failure-directions
  table, the path in/out table, the link case the rule does not decide, why HEAD
  is digested and the branch name is not, and **six known coverage gaps** now —
  the sixth, pipes and devices, was added by the hardening below.
  `CHECK_PIPELINE.md` step 3 and `FROZEN_SEMANTICS.md` (a new §"What 'the same
  project state' means") both point at it.

**What the hardening added after the acceptance, and why each one is a defect
rather than a precaution.** Three commits, all on top of an already-accepted
task, none of which changes a verdict for a project that is not hostile:

- **`c735a2f` — a path Git reports could climb out of the project.**
  `relative_to_root` stripped the prefix and joined the result onto the root
  without checking it, so a path containing `..`, a root or a drive prefix would
  have been read from outside the folder SURE was asked about. It is not
  reachable through a well-behaved Git — the pathspec is `-- .` — but the
  repository is untrusted input and its index is a file in it, which is the same
  reasoning that puts `--no-optional-locks` in the invocation. The same commit
  fixed the three CI failures above.
- **`9f13f0d` — a repository could make fingerprinting run a program, or hang.**
  `core.fsmonitor` names a hook Git runs, read from the repository being
  described; `Git::SAFETY_ARGUMENTS` now overrides it and `--no-pager`. And
  `File::open` on a FIFO with no writer **blocks until a writer appears**, so a
  project that replaced a tracked file with a pipe could hang a check with no
  output — a state indistinguishable from "still working". `Reader::read` now
  answers such a path by kind without opening it.

**What the mutation run found, because the green suite did not.** Three of the
five tests added by `P2-T002` exist because a mutation survived (the section
below has the detail): the nested-repository walk in `Reader::tree` was reached
by **no test at all**; the monorepo test proved only *stability*, so an
implementation that never stripped the prefix — and therefore never read any
file's contents — passed every assertion it made; and the byte budget had only
ever been exercised with one file, which cannot tell a per-file budget from a
per-fingerprint one. None of the three was visible as a failure.

## What `P2-T001` added

`crates/sure-core/src/scan/` — **the file list every later stage reads**, and the
place where a loss becomes invisible. Four modules plus a new integration target:

- `skip.rs` — `SkipReason` (ten variants) and `Skipped`. **A skip is a value in
  the result, not a log line**, which is the whole task: a scanner that returns a
  list of files has said nothing about the files it did not return.
  - `is_by_design()` and `loses_coverage()` are **two full `match`es, not one
    predicate and its negation**. A negation answers `false` — "not a loss" — for
    a variant nobody thought about, and that is the quiet direction. A new variant
    now fails to compile in both until both questions are answered.
  - `NotFollowed` and `SpecialFile` are **losses even though not looking at them
    is deliberate**: what a link points at is not in the scan, and a pipe is an
    entry that exists and is not in the list. Classifying them as "by design"
    because the scanner chose them would be the scanner grading its own decision.
  - `SureCache` (`.sure`) is separate from `Cache` for a reason none of the
    others have: a fingerprint taken over a tree containing SURE's own output
    changes *when SURE runs*, so checking a project would change the thing being
    checked.
  - The `.sure` rule is spelled through `paths::PROJECT_CACHE_DIR`, not as a
    literal, so the directory SURE writes into and the one it skips cannot drift.
- `ignore.rs` — two tables of exact names, **split by entry kind**. `build`,
  `target` and `dist` are both a tool's output directory and a hand-written
  script at the root; one table keyed on the name alone cannot tell those apart
  and drops the script. `bin`, `obj`, `out`, `Debug`, `Release` and `third_party`
  are deliberately **not** in either table — each is a plausible project
  directory, and leaving a real directory out of a scan is a loss.
  `matching_rule(name, kind, case)` takes the case rule as an **argument**, so
  both platform rules are testable on one machine; the fold is ASCII-only.
- `error.rs` — `ScanError`, the four refusals (`NotAbsolute`, `Missing`,
  `NotADirectory`, `Unreadable`). Each is *no scan at all* rather than a scan with
  losses, because an empty `Scan` and a project with nothing in it look the same.
  Each message names the path and says what SURE did instead; `Unreadable` quotes
  the operating system rather than paraphrasing it.
- `mod.rs` — `scan(root, ScanOptions)`, `Scan`, `Entry`, `EntryKind`,
  `display_path`, and the walker. **It reads no file contents** — names and
  file-or-directory only — and a source scan in `tests/scan_project.rs` asserts
  exactly that against the four files, because a scan of a project whose files
  are huge, encrypted, on a slow share or cloud placeholders that would be
  *fetched* by being read must cost the same as any other.
- `tests/scan_project.rs` — 30 tests over real filesystems, in scratch
  directories under `target/tmp/` whose *names* contain a space and a non-ASCII
  character, so every path in every test is a path the platforms disagree about.
  Links are built with `mklink /J` on Windows and `symlink` elsewhere.
- `docs/architecture/PROJECT_DISCOVERY.md` (new) — the three guarantees, the skip
  vocabulary, both tables and what is deliberately absent from them, the four
  refusals, determinism, and three **known coverage gaps**. `CHECK_PIPELINE.md`
  step 1 now points at it, and `FROZEN_SEMANTICS.md` gained four rows plus a new
  §Closed vocabularies are matched in full.

**The three guarantees, and how each is held.** *Stays inside the root*: children
are the parent's path joined with one `file_name()` from the operating system —
a single component, no separator, no `..` — and links are never followed. That is
an argument *from construction*, not a check that could be wrong, which is why
`SkipReason` has no `Outside` variant. *Is bounded*: `max_depth` (32) and
`max_entries` (200 000), each recorded when it bites; a directory of two hundred
thousand files does not hang SURE, it makes SURE say it stopped. *Says what it did
not look at*: `Scan::is_complete()` is the one question a caller must answer, and
`losses()` versus `scope()` separates "something may be missing" from "SURE said
it would not look there".

**Three decisions worth keeping.** `DirEntry::file_type()` reports a junction as
`is_symlink() == true, is_dir() == false`, so the walk's arms are matched
symlink-first; without that the `Some(_)` arm would file every junction on Windows
as a `SpecialFile` and the "never followed" guarantee would be nominal. The
ignore tables are consulted *after* that arm, so a link named `node_modules` is
truthfully reported as `NotFollowed` (a loss)
rather than `Vendored` (declared scope) — reporting the wrong reason is by itself
enough to turn a loss green. The root **is** followed through a link and is not
subject to the ignore tables: the caller named it, and a scan of a directory
called `target` scans it. And `.gitignore` is deliberately not consulted: a
project's ignore file is a statement about the repository, which is a different
question from what belongs in a check.

## What `P1-T011` added

`crates/sure-core/src/config/authority.rs` (new, 18 tests) — **the answer to
"the project's file asked for host execution; is that a yes?"**, which is a
question about two files rather than about one.

- `Layer { User, Project }` with `can_grant()`. Ranks 2 and 4 of
  `CONFIG_AUTHORITY.md` exist; **rank 3 (organization policy) deliberately does
  not**, and neither `Layer` nor `ConsentGrantor` offers a way to name it — a
  source a caller can name but never obtain is how a documented feature becomes
  a believed one. Rank 1 is a decision rather than a file and lives in
  `ConsentGrantor::InteractiveUser`.
- `Privilege { request, asked_by, granted_by }`. The refusal is a **value**, not
  an omission: a file that asked for network access and did not get it leaves a
  `Privilege` behind, so a report can say what was asked for. Dropping it would
  make "the project asked and was refused" and "the project asked for nothing"
  the same list.
- `Resolved<T> { value, by: Option<Layer> }`. `by: None` means "nothing beyond
  the default", not "SURE did not work it out". Protection and privacy resolve
  to the **stricter** value either layer set, and `by` names the most trusted
  layer that asked for it.
- `Authority::load` / `new` / `privileges` / `privilege` / `permissions` /
  `protection` / `privacy_mode`.

**There is no merged `Config` and no `Authority::effective()`** — a merge was
rejected by ADR 0011 because it cannot be reported back in terms of the files the
user wrote. `Authority::permissions()` answers "which permissions a *file* was
allowed to hand over", which is a different question from
`sure_domain::execution::decide`'s "may this action run"; the execution **mode
is not a permission**, so a project asking for `host_confirmed` gets nothing in
the permission set.

`docs/architecture/CONFIG_AUTHORITY.md` was rewritten around what exists: the
order and the may/may-not lists are kept, and **"Nothing routes through it yet"**
is stated plainly — no command builds an `Authority` today, and wiring it in
front of the check pipeline is **P13-T009**. Until then a report that claimed a
project's request was refused would be describing behaviour that has not run.

Two smaller changes: `Config::load_file(path)` splits from `Config::load(root)`
so the user's own file is read by the same reader (the near-miss `sure.yml` check
now follows the requested *file name*, not its directory), and
`neither_routes_through_the_authority_yet`-style honesty in the docs.

**The one test that could not be written any other way** is
`both_files_are_read_from_where_they_were_asked_for`, which goes through
`Authority::load` with two real files. An `Authority` that found the user's file
and silently discarded it passes every test that builds one directly — proved by
mutation before the test was written.

## What `P1-T010` added

The protocol version handshake, in one function used by two callers:

- `crates/sure-protocol/src/handshake.rs` (new) — `Handshake { Agreed,
  CallerIsOlder, CallerIsNewer }`, `negotiate(u32) -> Handshake`, the three
  accessors, and `Display`. **The whole version rule.** The module documentation
  says why the rule is exact equality, why there is no compatibility table, and
  why the two directions are not the same answer.
- `crates/sure-protocol/src/event.rs` — `from_json` calls `negotiate` rather
  than comparing numbers, so the reader and the CLI cannot drift. The test
  `the_reader_and_the_handshake_refuse_the_same_versions` drives both over
  `0..=PROTOCOL_VERSION + 3`.
- `crates/sure-core/src/lib.rs` — re-exports `Handshake` and `negotiate`.
  `sure-cli` has one edge into the engine (ADR 0001, and the note in its
  manifest), so a new protocol type is re-exported rather than added as a
  second dependency.
- `crates/sure-cli/src/cli.rs` — `Protocol` grows `--speaks VERSION`.
- `crates/sure-cli/src/commands.rs` — one more arm; with `--speaks`, the answer
  is `sure_core::negotiate`'s.
- `crates/sure-cli/src/report.rs` — `Report::Handshake`, and with it the two
  predicates: `outcome` is `ok` or `unavailable`, `exit_code` is 0 or **3**, and
  `is_an_answer` is `handshake.is_agreed()` — so the agreed sentence goes to
  stdout and the refusal to stderr, by the same predicate as every other
  command. The frame carries `sure_speaks`, `caller_speaks`, `agreed` and
  `update`, never the sentence.
- `crates/sure-cli/tests/cli_contract.rs` — a process-level test that reads the
  version out of `sure protocol --format json`'s own frame and then requires
  that exact number to be agreed and the two neighbouring ones refused, in both
  directions. It carries no copy of the version, so it cannot keep passing after
  what SURE announces and what it accepts have drifted apart.

`docs/architecture/PROTOCOL.md` gained §The handshake and lost two of its three
known gaps; `docs/architecture/CLI.md` gained §`sure protocol`.

**What this does not cover.** The acceptance criterion names CLI, hook and MCP
adapters. The hook and the MCP server are commands this build does not
implement, so nothing but a caller's own shell has run the handshake. The rule
is reachable, and `PROTOCOL.md` §Known gaps now says which half is missing
rather than implying the whole of it is done.

## What `P1-T009` added

`sure doctor` — the first command whose answer depends on what it found, and so
the first place where "the run was fine" and "the answer was fine" come apart.

- `crates/sure-core/src/doctor.rs` — `examine_this_machine()` returns a
  `DoctorReport` **value** (paths, presence, store facts, one tool, problems,
  and what it did not check), so a test can build one and a renderer can read
  it. Exit 0 when `is_well()`, **1** when it is not: the false-green rule applied
  to SURE's own installation, so `sure doctor || fix it` works.
- `crates/sure-cli/src/doctor.rs` — the human and machine renderers, separately.
- Three things it deliberately does not do, each structural rather than
  promised: it does not read the settings file (no field could hold a secret,
  and a source scan in `tests/doctor.rs` enforces it), it does not create the
  store (presence checked before `Store::open_at`), and it does not run the
  programs it finds.

## What `P1-T008` added

`crates/sure-cli/` — the command surface, and the two ways a result reaches a
person and a script:

- `src/cli.rs` — the grammar as one `enum Command` plus `HistoryAction`,
  `ConfigAction`, `HookAction`. `docs/architecture/CLI.md` lists the same ten
  commands in the same order, and a test compares the doc against what this
  build parses.
- `src/report.rs` — `report::exit` (the whole status table, the only place a
  status is chosen), `NotYet`, and `Report { Version, Protocol, Unavailable }`
  with `human`, `machine` and `frame`.
- `src/output.rs` — `Format` and the two output paths. **The only module in the
  crate that names a process stream.**
- `src/commands.rs` — `Command::report`, one exhaustive match with no `_` arm
  and no `unreachable!()`. Adding a command to the grammar fails the build here
  until somebody decides whether this build implements it.
- `src/main.rs` — `try_parse` rather than `Parser::parse`, so clap's exit status
  goes through `status_of` and the documented table stays SURE's. 22 unit tests.
- `tests/cli_contract.rs` — 11 process-boundary tests over the built binary.
- `docs/architecture/CLI.md`, and two amendments to `PROTOCOL.md`.

`clap` 4.6.6 is `sure-cli`'s first dependency, named by `RUST_DESIGN.md`.

**Two commands work: `version` and `protocol`.** Both answer questions about SURE
rather than about a project, which is why they need no engine. Every other
documented command parses its arguments, decides it cannot do the job, exits **3**
and says so in one sentence. Nothing is half-done; nothing that did nothing exits
0. `docs/architecture/CLI.md` §Exit statuses is the table, and it marks two
statuses (1 and 4) as reserved before anything returns them so a script written
against this release is not invalidated later.

## What `P1-T005` added

`crates/sure-core/src/store/` — one SQLite file, at `Paths::store_file()`:

- `mod.rs` — `Store`, `StoreOptions`, `HistoryFilter`, the five-point concurrency
  contract, redaction-then-validation on the write path, and the one bounded
  retry in the module (`establish_journal_mode`).
- `migrations.rs` — `PRAGMA user_version`, one transaction per migration, an
  append-only `MIGRATIONS` list, and refusals for a newer file, a foreign file
  and a version that is not a version.
- `sql/0001_records.sql` — one `STRICT` table with `AUTOINCREMENT` and three
  indexes. `include_str!`, so an installed `sure.exe` migrates against what its
  code was built from.
- `record.rs` — `RecordKind` (the six documents plus `Recording`) and
  `StoredRecord`, which carries `document_version` and refuses a row from a newer
  build. This closes `FROZEN_SEMANTICS.md` conformance gap 3 and `PROTOCOL.md`
  known gap 1.
- `error.rs` — `StoreError`, every message saying what SURE did instead.
- `tests/store_concurrency.rs` — real child processes, with a barrier.
- `tests/store_packaging.rs` — `bundled`, no async runtime, no unsafe.

`docs/architecture/STORAGE_AND_DATA_PATHS.md` gained a §The store.

## Two real bugs this task found, both by mutation-checking a green test

Recorded because both are the kind of thing that ships silently.

1. **`PRAGMA journal_mode = WAL` bypasses the busy handler.** SQLite's
   `sqlite3_busy_handler` documentation names the journal-mode change as a case
   where it declines to invoke the handler, because waiting could deadlock. Five
   of six processes opening a *fresh* database therefore died with
   `database is locked` before reaching a single write — reported as
   `StoreError::Open`, so the message said the file could not be opened rather
   than that anyone had contended. Fixed by `establish_journal_mode`, which waits
   for the same `busy_timeout` and reports `StoreError::Busy`.
2. **`Store::write_error` reported `DEFAULT_BUSY_TIMEOUT`, not the configured
   one.** A store opened with a 50 ms timeout told the user "SURE waited 5000 ms",
   which is a false statement about what just happened with no way for the reader
   to tell.

Neither was visible until `tests/store_concurrency.rs` was made to *fail*: the
first version of that test passed with the bug present, because spawning six
processes takes longer than migrating a database and the children never
collided. The barrier (`a_moment_from_now`, an 800 ms spin) is what made the
contention real. **A test that cannot fail is worse than no test, because it is
read as evidence.**

## Adversarial (mutation) verifications on this branch

Each was reverted after confirming the check fires.

### `P2-T007`

`target/tmp/mutate11.py` (git-ignored), **20 mutations, 20 observable, 0 declared
unobservable**, exit 0: *"all 20 observable mutations caught by a failing test,
and 0 declared unobservable as expected"*.

Four families, and the second is the one the task's second acceptance criterion
is about:

- **A stack SURE did not read, presented as one** (9). `Partial` and `Unknown`
  widened into `Read`; `is_read` answering `true` everywhere; a member whose
  manifest *was* read reported as one that was not.
- **"I did not look" presented as "there are none"** (3). Python's unread member
  list answered as `NoWorkspace`; `is_known_single` answering `true` for
  everything; a truncated member list reported as complete. Each turns a caveat
  into a finding.
- **One place reported as two, or as the wrong place** (5). The merge removed so
  a directory two ecosystems name becomes two components; containment decided by
  the text of a path rather than by its path components; containment to the
  outermost rather than the nearest component; the root contained by itself; the
  components left in the order the ecosystems named them instead of sorted by
  path.
- **An absent fact invented, or a real one dropped** (3). A root manifest
  reported as read whatever it was; an ecosystem that was never found reported as
  one that read a declaration and named nobody; `root_component` handing back the
  last component.

**The first run was not clean, and the three holes it found were real.** Three
mutations came back `MISSED`:

1. *a pyproject.toml SURE could not read is reported as no manifest* —
   `python_root_reading` has an arm that prefers a recorded failure over a
   missing file, and nothing tested it. Fixed by
   `a_pyproject_toml_that_could_not_be_read_is_unread_rather_than_no_manifest`.
   The Node case had a test; the Python case had none, and the Python case is the
   one with two root files and a preference rule between them.
2. *a member list that was cut short is reported as complete* — nothing set
   `max_workspace_members` low enough to observe truncation. Fixed by
   `a_member_list_that_was_cut_short_is_not_reported_as_complete`.
3. *the root is reported as contained by itself* — every containment test found
   edges **by looking one up**, so an extra edge nobody asked about was invisible.
   Fixed by asserting the total: `contains.len() == members().count()`.

The third is the one worth remembering: a test that looks an edge up by its key
cannot see a duplicate, and a test that asserts the count can. The first two are
the same shape as each other — a rule with a dedicated arm and no test aimed at
that arm, in a module where every other arm had one.

One mutation was written and then **replaced before it ran**: *a dependency
naming a workspace package is turned into a component* was a no-op edit that
would have come back `MISSED` and proved nothing, because a mutation can only
replace text and this bug needs code added. A mutation that cannot fail is not
evidence, and leaving it in would have produced a `MISSED` that looked like a
hole in the tests rather than a hole in the harness.

The harness carries `mutate10.py`'s three counting rules unchanged — a
non-unique anchor is `SKIP` and counts as nothing, a compile error is `BUILD` and
never `CAUGHT`, and **the suite is run unmutated first and a red baseline ends
the run**. The baseline guard is copied into this file rather than shared,
because a harness that depends on another harness is one more thing that can be
missing when it is needed.

### `P2-T006`

Thirty-four mutations in `target/tmp/mutate10.py` (git-ignored), run against
`cargo test -p sure-core --lib --test discover_rust --test discover_node --test
discover_python --no-fail-fast`. **Thirty-two CAUGHT, zero MISSED, zero SKIP,
zero BUILD**, and **two declared unobservable**, which print `MISSED*` and do not
fail the script. The script's own last line is the verdict, and it reads *"all 32
observable mutations caught by a failing test, and 2 declared unobservable as
expected"* with exit code 0. The figures here are the re-run **on the tree as it
stands after the `pattern.rs` tests**, after the last edit to either the module or
the script, and the reverted tree was checked afterwards (`git status` shows only
the files this commit touches).

**The baseline is now checked by the script, and that is the most important line
in it.** The run before this one reported `DECLARED UNOBSERVABLE BUT CAUGHT` for
both of its declared-unreachable mutations — which should be impossible, and was
not: the suite was **already failing** before the first mutation was applied, so
every mutation "failed a test" and the verdicts meant nothing. The cause was the
`pattern.rs` scratch helper's use of the process id (see "What the second
`P2-T006` commit added"). A mutation cannot be credited with a failure that was
there before it, so the script now runs the suite unmutated first and stops with
`BASELINE IS NOT GREEN` if it is not clean. Checking that by hand — which is what
was done before this run — is a step a person can forget, and it was forgotten.

**The second change is the same rule applied where the code had not followed
it.** A mutation that stops the code compiling is `BUILD`, but the FAILED lines
and the compiler-error lines were collected into one list, so an `error:` line
reaching stdout would have been enough to return `CAUGHT`. The two lists are now
separate, and only a failing *test* can produce a catch. The rule was written
first and the code merged its two cases; this is the rule with the merging
removed.

The families are the three the module is arranged around, and every mutation is a
plausible **wrong implementation of detection** rather than a random edit:
*"not there" arriving as "there but unreadable"* and the reverse (1–8); *an
invented fact* (9–18); and *a weaker source overriding a stronger one, or a rule
applied that SURE cannot cite* (19–23). The last four cover what counts as a Rust
project at all and the two lint tools Cargo defines. A **fourth family (28–34)**
is anchored on `pattern.rs` rather than on `rust.rs`, so the tests added in the
second commit are shown to be load-bearing rather than counted as coverage: the
containment rule (28–29), `*` naming directories and not files (30), the root
never being its own member (31) and never being named twice (32), a refusal told
apart from a pattern that named nothing (33), and the `Normal`-only guarantee
(34, declared unobservable).

**Two mutations revert a decision this task made and documented, which makes them
the most valuable here: they are the bugs that were designed out rather than
discovered.** Mutation **24** reads `[workspace]` from the wrong key — the exact
shape of the first draft of `from_json`, where the workspace tables hung off
`PackageSection`, so a virtual manifest's *entire* workspace would have been
dropped. Mutation **23** puts a `cargo` command back on a project whose manifest
SURE never read — the first draft of `command_for` gating on `package()` rather
than on the document. Both CAUGHT.

**The script found six problems on its first run, and five of them were real test
gaps closed with five new tests rather than reclassified:**

- a **member whose `Cargo.toml` is a directory**, not a file, was reported as a
  member with no manifest — a false statement about the project wearing a
  negative finding's clothes;
- a **toolchain file that could not be read as a file** — the existing test
  reached the `toolchain_from_text` error arm and never the `ReadFile::Unread`
  arm, so a `rust-toolchain.toml` over `max_manifest_bytes` was reported as a
  project that pins nothing;
- a **non-string in a `members` list** was turned into a directory name, so
  `members = [1]` invented a member called `1`;
- a **value read with the whitespace the file carried**, so `name = "  app  "`
  reached a finding with its spaces;
- a `Cargo.toml` that failed to parse could be reported as declaring nothing.

**The sixth is genuinely unreachable and is declared rather than hidden.** The
arm for a `Manifest::from_json` failure cannot be reached from a file: `read_toml`
parses with `toml::from_str::<toml::Value>`, and a TOML document **is** a table,
so `to_json` always yields `Value::Object` and the guard never fires for a caller
that went through the walk. The guard is pinned directly instead, by
`a_document_that_is_not_a_table_is_not_read_as_an_empty_manifest`, which hands
`from_json` the four non-tables a **direct** caller could pass. It stays rather
than being deleted because deleting it would make `from_json` answer a non-table
with an empty manifest — the false green this module is arranged against — the
moment any future caller reaches it by another route. **The second declared-
unreachable mutation is the matching half in `pattern.rs`** (34): the
`Component::Normal` arm of `expand` is unreachable because `contained_relative`
returns only `Normal` components, so `continue` and `return Err` behave the same —
until mutation 28 breaks the first half, which is why 28 is caught and 34 is not.
Both arms stay for the same reason: each is a guarantee stated where a future
caller can read it instead of trusted to a function in another module.
The script also reports the opposite error: an `UNOBSERVABLE` entry that starts
being `CAUGHT` is printed as `DECLARED UNOBSERVABLE BUT CAUGHT`, because it means
the list is now wrong — and that line is what exposed the run whose baseline was
red, so the check it was written for was never the check that made it useful.

**`BUILD` does not count as caught, and the rule earned its keep immediately —
against this author.** An edit made while writing the tests used
`manifest.package()` where `package` is a field, which broke the lib test target,
and the harness reported **all 27 mutations as `BUILD`**. Had `BUILD` counted as
caught, that run would have read as full coverage of a suite that did not compile.

**Mutation 7 needed a test edit rather than a test, and that is recorded because a
mutation caught by a rule other than the one it removes has not been tested.** The
toolchain-shape test's list already held `channel = "stable"` *with spaces*, but
that spelling is rejected by the bare-form line-count check even when the
`WrongShape` guard is deleted — so the mutation was being caught by a different
rule than the one it removes. `channel="stable"`, one token, which **only** the
guard can reject, was added to the list.

**Two stale cross-references in the script's own docstring were corrected against
a mechanical numbering of the list** rather than by eye: it cited *"mutations 1–8,
28"* where the list has 27 entries and no 28, and called the workspace-from-the-
wrong-key mutation 13 where it is 24. A scratch file that will be cited as
evidence is a document, and a document with a wrong number in it is the thing this
section keeps finding.

### `P2-T005`

Twenty-three mutations in `target/tmp/mutate9.py` (git-ignored), run against
`cargo test -p sure-core --lib --test discover_python --test discover_node
--no-fail-fast`. **Twenty-two CAUGHT, one MISSED, zero SKIP, zero BUILD**, and
the one missed is recorded with its reason and a verdict of *right*.

**The count is a re-run, and it corrects two numbers written earlier in this
session.** The first report said 22 mutations and 21 caught. Both were wrong by
one, in the same direction: the figures were written from the script as it stood
before its last mutation was added, and were never re-counted against the file.
`grep -c '^CAUGHT'` over the re-run's output is where 22 comes from, and the
script's own entry count is where 23 does. **It is recorded rather than quietly
fixed because a mutation count is precisely the sort of figure a later reader
treats as measured** — and because the error is the one this repository keeps
finding: a number that was true of an earlier state, carried forward past the
change that invalidated it. The reverted tree was checked afterwards (`git
status` clean of everything but `progress/state.json` and `progress/HANDOFF.md`,
which are this commit), so no mutation was left applied.

The run is the same three families `P2-T004`'s was, which are the three the
module is arranged around:

- **"Not there" arriving as "there but unreadable", and the reverse (1–6).** A
  `pyproject.toml` that failed to parse reported as a project with no
  `pyproject.toml`; a file SURE could not read absent from the result; a
  `setup.py` on its own, or a lockfile on its own, not making it a Python
  project.
- **An invented fact (7–11).** A URL read as a distribution called `https`; a
  build *library* named as the project's installer; a build plan offered to a
  project that declared nothing that builds it; a role with no declared tool
  given a command anyway.
- **A weaker source overriding a stronger one (12–15).** A requirements file
  deciding the installer over a `uv.lock`; two lockfiles resolved to the first
  instead of reported.

Plus bounds, reading, the level and the build plan's command (16–23).

**The script found one real hole in a suite that was already green, and it is
the same shape as `P2-T004`'s.** *"A project that declares no manifest is graded
as fully understood"* — changing the "recognised, but nothing SURE reads declares
anything" arm's level from `InspectOnly` to `Generic` — **passed every test**. So
a project SURE had read nothing from, a bare `uv.lock` or a bare
`.python-version`, was being reported as a project SURE fully understands. That
is the false-green direction exactly. **The mutation is now CAUGHT**, by
`a_project_with_no_manifest_sure_can_read_is_not_called_fully_understood`, which
also asserts the reason carries no project text.

`target/tmp/mutate8.py`'s two counting rules were kept, and both were earned
again in this run:

- **Two mutations came back `SKIP` because `rustfmt` had reformatted the block
  after they were written** — the anchor appeared zero times. An anchor that does
  not match exactly once counts as **nothing**, not as caught, which is what
  stops a report of a false green being itself a false report. Both were
  rewritten against the real text and both are now caught.
- A mutation that stops the code compiling prints `BUILD` and is not counted as
  caught.

**One mutation is not caught, and the verdict is right.** *"A plan with no
command still names tools as its evidence"* replaces a reset of `because` with a
plain rebinding and no test notices — because the guard is currently
unobservable: `CommandRole::Install` is the only role whose reasons can be
non-empty while its command is `None`, and `Install::tool_roles()` is `&[]`. It
stays because it makes the invariant hold for a **new** role by construction
rather than by the next author noticing, which is the standing
`FINGERPRINTING.md` gap 8 records for its sort and its domain tag. Recorded
rather than papered over with a test that would have to invent a role to reach
it.

**One mutation is deliberately absent, and the script says so.** *"`looks_like_one`
admitting a bare `.py` file"* is not expressible as an edit to it: its parameters
are six booleans and lists built from named files, so the edit would need the
markers returned by the *walk*, and `requirement_candidates` is the only place
`child_files` is consulted. Mutation 3 puts an arbitrary file into that candidate
list — the closest expressible form — and the marker list itself is covered by
two tests that do not depend on that function:
`every_file_that_marks_a_python_project_is_enough_on_its_own` (the integration
test, which writes each marker into a fixture and requires the project to be
recognised) and `.._is_on_the_list_that_decides` (the unit test, which checks the
list against the markers the walk can produce).

### `P2-T004`

Twenty-seven mutations in `target/tmp/mutate8.py` (git-ignored), run against
`cargo test -p sure-core --lib --test discover_node --no-fail-fast`. **All
twenty-seven applied mutations were caught; none was reported MISSED, BUILD or
SKIP in the final run.**

Each is a plausible wrong *reading* rather than a random edit, and the direction
that matters is the false green. Grouped by what they attack:

- **Package managers (1–5).** The lockfile is invisible; two lockfiles are read
  as agreement; a disagreement is answered anyway; an unrecognised manager is
  guessed at; an `engines` range is ranked as strongly as a lockfile.
- **Workspaces (6–11).** The root joins its own workspace; a pattern SURE cannot
  expand is answered as one that named nothing; `**` is expanded as a single `*`
  instead of being refused; a truncated list keeps quiet; the pnpm workspace file
  is read and then ignored.
- **Scripts (12–13, 16).** A conventional role with no script vanishes from the
  rows; a script with no command is dropped rather than reported; npm is made to
  use `run` for the two scripts it does not.
- **Dependencies (14–15).** The one that found the hole — see below.
- **The enum rule (17–24).** These are the point of the task, and each makes "I
  could not read this" arrive as "there is nothing here": a link at a manifest's
  name becomes an absence; a budget refusal and a shape failure live in the state
  but not in the result; a file over the byte limit is read from the part that
  fitted; an unread manifest is graded as one SURE can read.
- **The rest (25–27).** The conclusion stops naming its files; a climbing path is
  kept rather than refused; the stack for a level is reported as a new word
  instead of the domain's.

Two mutations are **deliberately absent** and the script's docstring records why,
rather than leaving a reader to assume the run was exhaustive: substituting a tool
name is not expressible, because `collect_tooling` only pushes a row when the
dependency's name already equals it, so there is nothing to substitute; and the
tooling sort cannot be observed from one run, for the same reason `P2-T003`
recorded for the fingerprint's sort — one run sees one filesystem's order.

Three anchors were rewritten after the first run, each for a different reason and
each a real hazard worth naming: one anchor matched **two** identical probes and
was extended until it matched once; one mutation removed a binding's only use and
therefore **did not compile**, which the script reports as `BUILD` rather than
counting as caught; and one was a **no-op** that would have read as a clean miss.

The script reports `SKIP` for an anchor that does not appear exactly once and
`BUILD` for one that does not compile, and neither is counted as caught. **That
distinction is what made the one real finding visible**: *"a dependency with no
range is dropped instead of reported"* came back **MISSED** — not skipped, not a
build failure. The field simply had no test, and the mutation that deletes it was
invisible to a suite that was entirely green.

### `P2-T002`

Twenty mutations at the acceptance, **twenty fired**; **twenty-three at
`9f13f0d`, all twenty-three caught**, plus one applied and reported `BLIND`, in
`target/tmp/mutate6.py` (git-ignored). Every one is a plausible *wrong
implementation of fingerprinting*, not a random edit, and most make the
fingerprint ignore something it must not — the false-green direction, and the
direction the two acceptance criteria are about.

The script got two things right that the earlier ones did not, both because of
failures in its first version:

- **A mutation that stops the code compiling is now reported as `BUILD`, not as
  caught.** A non-zero `cargo` exit with no `FAILED` line says nothing about
  whether a test would have noticed the behaviour, and counting it as caught is
  the same false green this script is looking for, one level up. Two mutations in
  the first run were exactly this.
- **An anchor that does not match exactly once prints `SKIP`.** The first version
  of "HEAD is not part of the fingerprint" still hashed HEAD — a no-op mutation
  that was duly reported MISSED, i.e. a hole that did not exist. A mutation
  reported as missing when it was never applied is a false report of a false
  green, which is worse than either.

The catches worth naming:

- **HEAD not digested**, **the branch name digested**, **a clean project's empty
  change list digested like a full one**, **an untracked file dropped**, **a file
  deleted made equal to a file emptied** (using the literal SHA-256 of the empty
  string), **a file's own path left out of its digest**: each fails at least one
  test, and together they are the first acceptance criterion.
- **The ignore tables not applied**, **a walk that lost something hashed
  anyway**, **a file that moved inside the walk keeping its place**, **the
  project's place inside the repository ignored**: the second acceptance
  criterion, and the errors-not-partial-fingerprints rule.
- **Both budgets** — the file budget not enforced, and the byte budget applied
  per file instead of per fingerprint.
- **Three about what Git is asked**: `--relative` added after all (the flag that
  silently prints nothing on Git 2.55.0), `--no-renames` dropped, and
  `--untracked-files=normal` instead of `all` — the last being the one that
  makes a nested checkout one opaque line.
- **Two refusals**: a relative root quietly resolved against the working
  directory, and a Git that will not start reported as a Git that failed. The
  second matters because the first message says "install Git" and the second says
  "Git ran and refused", and a user told the wrong one fixes the wrong thing.
- **Two about the framing itself**: fields written without their length (`ab`+`c`
  = `a`+`bc`), and an absent field written as an empty one.

**Three real holes, found by mutations that were green**, and the tests written
to close them:

1. **`Reader::tree` — the whole nested-repository walk, the per-file path digest
   inside it, and the `IncompleteTree` guard — was reached by no test at all.**
   `a_directory_git_will_not_descend_into_is_walked_and_read` builds a real
   nested `git init` inside the fixture; probing Git confirmed it is reported as
   one untracked directory (`? inner/`) even under `--untracked-files=all`, which
   is precisely why the walk exists. It asserts the change of a file inside the
   nested checkout, the addition of the nested checkout, and that a file moved
   *within* the walk changes the fingerprint.
2. **The monorepo test proved stability, not correctness.** It asserted that a
   fingerprint is unchanged when nothing changes, so an implementation that never
   stripped `--show-prefix` — looking every path up at `<root>/<prefix>/…`,
   finding `Gone`, and hashing nothing — passed every assertion it made.
   `a_change_inside_a_project_that_is_not_the_repository_root_is_read` now edits
   the same file twice and requires the two fingerprints to differ, which is the
   only form of that test that reads a file.
3. **The byte budget was only ever exercised with one file**, so per-file and
   per-fingerprint were indistinguishable.
   `the_byte_limit_is_over_the_fingerprint_and_not_over_each_file` writes two
   600-byte files under a 1 000-byte limit and requires the error to name the
   file that crossed it.

**What this run does not cover, stated rather than implied.** The link and
unreadable-file paths are `#[cfg(unix)]` in the test file, so no mutation was
applied to them on this machine — the script's own header says so. That is not
coverage; it is the reason the `ubuntu-latest` and `macos-latest` CI jobs exist.
And the mutation list is a list: twenty-three plausible wrong implementations,
not the space of wrong implementations.

**The Unix-only entry is now applied and reported `BLIND`, not omitted.** At
`9f13f0d` the script gained a second list, `UNIX_ONLY`:

```
BLIND   a pipe is opened instead of being described: as expected
NOT OBSERVABLE ON THIS PLATFORM (1), so unverified here:
  - a pipe is opened instead of being described
```

It is applied and run anyway, so a stale anchor or a mutation that no longer
compiles is still caught here; but it is reported as neither `CAUGHT` nor
`MISSED`, because both would be a claim about a code path that did not execute.
`MISSED` would be the quiet lie and `CAUGHT` the loud one — a Windows test
cannot have noticed a Unix-only behaviour, so a failure there would mean
something *else* broke. Omission was the third option and the worst: the earlier
header described the gap in prose, and a reader skimming twenty-three `CAUGHT`
lines does not see it.

Two entries were added at `9f13f0d`, both caught:
**`core.fsmonitor=false` dropped from `Git::SAFETY_ARGUMENTS`**, and
**`--no-pager` dropped instead**. They are worth naming because on every
repository that does not exploit them the fingerprint is *identical* with and
without them — the only failing test is the one that reads the constant, which
is the entire reason the constant exists rather than the arguments being written
inline at the call site.

### `P2-T001`

Twenty-eight mutations, **twenty-seven fired, one did not**, in
`target/tmp/mutate5.py` (git-ignored). The one that did not is recorded in
`PROJECT_DISCOVERY.md` §Known coverage gaps rather than deleted from the list: a
mutation that escapes because its *input cannot be built* is a gap, and a
mutation quietly removed from the list is a gap nobody knows about.

Six are the false-green shapes this task exists to prevent:

- **`is_complete()` returning `true` unconditionally**, and separately **a loss
  reason moved into the "by design" group** and **a declared skip moved into the
  loss group**. The first two fail a dozen tests; the third fails
  `every_reason_answers_both_questions_consistently`, which is the test that
  exists because the two predicates must not be each other's negation.
- **A link filed under whatever name it has** (the ignore table consulted before
  the symlink arm) fails `a_link_is_a_loss_whatever_it_is_called`. **This test
  was written because the mutation found the hole**, not the other way round: the
  first version of the link test used a link called `shortcut`, which is not an
  ignore-table name, so the ordering the module comment claims was unenforced.
- **A link reported as `Vendored`** — the same false green by the other route —
  fails the same test.

Three found real holes and two were equivalent mutants, which is the more useful
half of the result:

- **A rule matched against the relative path instead of the name** — i.e. "the
  ignore tables apply only at the top level" — was **green** until
  `a_left_out_directory_is_left_out_at_any_depth` and
  `a_nested_version_control_directory_is_left_out_too` were added. Every existing
  fixture had `node_modules`, `target` and `.git` at the root, so a scanner that
  would read a vendored tree in a real workspace passed the whole suite.
- **Sorting by `to_string_lossy()` instead of by the `OsString`** was **green**
  until the file-name test below was written. The comment in `mod.rs` claimed the
  text sort loses the order; nothing tested it.
- **The Unicode fold instead of the ASCII fold** was an **equivalent mutant**
  through `matching_rule`: no name in either table contains a letter with a
  non-ASCII lowercase twin (the kelvin sign is the only Latin one, and no rule
  contains a `k`). It is caught now by
  `the_fold_is_ascii_rather_than_the_one_unicode_defines`, which calls the private
  `name_matches` with a rule named `kotlin-build` precisely because going through
  the tables cannot see the difference.
- **Dropping a `read_dir` entry the operating system failed to describe** is the
  one that escapes. See the gap note above.

### `P1-T011`

Fourteen mutations, fourteen fired. The script is `target/tmp/mutate4.py`
(git-ignored) and it prints `SKIP` loudly when an anchor does not match — see the
`P1-T009` note below for why that matters more than the pass count.

Three are the false-green shapes this task exists to prevent, and they are the
ones worth keeping:

- **A project layer granting itself execution authority** — `Layer::can_grant`
  answering `true` for `Project` — fails
  `a_project_file_cannot_grant_itself_anything`,
  `only_the_user_layer_can_grant`, and the refusal-count assertions. This is the
  whole acceptance criterion in one line.
- **A refusal dropped instead of reported** — filtering refused privileges out of
  `privileges()` — fails
  `a_refusal_keeps_the_request_that_was_refused`. The distinction between "asked
  and refused" and "never asked" is the reason `Privilege` is a struct and not a
  `Vec<ProjectRequest>`.
- **The permission set starting with the network already allowed** fails the
  test that builds the set from `inspect_only()` and the test that requires an
  ungranted request to change nothing.

Two more are worth recording because they were **holes the mutations found
rather than confirmed**:

- **`resolve` keeping the *last* stricter value on a tie instead of the first**
  was green until `when_both_layers_ask_for_the_same_thing_the_user_is_named`
  was added. A tie silently named the *project* as the reason a restriction
  exists, which is exactly backwards.
- **`near_miss_beside` keying off the directory rather than the file name** was
  green until `the_near_miss_check_follows_the_file_name_not_the_directory` was
  added — the same class of bug, and the reason that test exists at all.

### `P1-T010`

Six mutations, six fired. The script is `target/tmp/mutate3.py` (git-ignored).

- **`negotiate` agreeing with any version at or above this build's** fails five
  tests across `handshake.rs`, `event.rs` and one already-written
  `round_trip.rs` test. The cheapest plausible wrong rule, and the most
  damaging: an adapter would be told yes and send events SURE then misreads.
- **The two directions swapped** fails `a_mismatch_says_which_side_has_to_move`
  and nothing else — which is the point of that test. A caller told the wrong
  direction retries with the fix that cannot work.
- **A refused handshake exiting 0** is the false-green mutation and fails three
  unit tests plus the process-level one. It is the check this task exists for.
- **The CLI answering every caller with its own version** fails both the unit
  test that compares the CLI's answer to `negotiate`'s and the integration test.
- **`is_an_answer` true for a refused handshake** fails two unit tests and the
  integration test's `stdout.is_empty()`, so the complaint cannot be moved onto
  the stream a caller is reading an answer from.
- **The event reader comparing versions itself** — written so that it accepts an
  *older* version the handshake refuses — is caught by
  `the_reader_and_the_handshake_refuse_the_same_versions` **and by that test
  alone**. Worth recording: the first version of this mutation accepted a
  version *above* the tested range and escaped, which is a property of the
  range, not of the test. A mutation outside the range it wrote is not evidence
  about it.

### `P1-T009`

Six mutations, six fired. Two are worth keeping:

- **Making the store open before the presence check** — a diagnostic that
  creates what it diagnoses — fails `tests/doctor.rs::the_report_never_creates_what_it_reports_on`
  and the `sure-core --lib doctor` tests.
- **Putting `version_string()` back into `Build.version`** reproduces the
  `SURE SURE 0.0.0-bootstrap` defect. It was found by reading output, not by a
  test, so the test came after; that is the wrong order and it is worth saying
  so.

The first attempt at the store-creation mutation reported `SKIP` because the
replacement text did not match the file — `target/tmp/mutate2.py` was rewritten
against the real `match presence(store_file)` text. **A mutation that reports
SKIP has tested nothing, and a script whose only output is "all fired" will say
that about a mutation that never applied.** Both scripts print SKIP explicitly
for this reason.

### `P1-T008`

- **Making a refusal exit 0** fails `main::tests::a_bare_sure_is_not_a_success`
  and `commands::tests::…` in the unit tests, and three in `cli_contract.rs`.
  This is the false-green rule inside SURE's own front door.
- **Flipping `Report::is_an_answer`** so a complaint went to stdout fails one
  unit test and two integration tests. **Visible only under `--no-fail-fast`** —
  without it `cargo test` stops at the first failing target and the integration
  file never runs, which is why the gate command carries the flag.
- **Adding a stray `println!` outside `output.rs`** fails four tests, including
  `only_the_output_module_writes_to_a_stream`. Checked by mutation because a
  source scan is the only thing that can catch an *absence*; no run of the binary
  demonstrates that a line was never written.
- **Reading standard input in `hook ingest`** fails
  `hook_ingest_does_not_read_standard_input`, which writes a megabyte into the
  pipe and requires a broken pipe. A command that drained the event and then
  refused would have destroyed the evidence it was refusing to record.
- **Adding a `Command` variant** produces `E0004` in two places (`Command::name`
  and `Command::report`), so a new command cannot ship with no answer about
  whether this build carries it out.
- **Making `frame()` report the wrong command name** fails
  `cli_contract.rs::the_machine_form_is_one_object_on_one_line_of_standard_output`
  at line 213. Worth recording *which* test caught it: the unit test beside it
  compares the frame against `report.command()`, so it stays green under this
  mutation — it is self-consistent by construction. **The integration test is the
  one that is load-bearing for the frame's content.**

### `P1-T005` and earlier

- Removing `features = ["bundled"]` from the workspace `rusqlite` entry fails
  `store_packaging::sqlite_is_compiled_into_sure…`. Worth knowing *why* the test
  exists rather than leaving it to the build: without `bundled`,
  `cargo build -p sure-core` still **succeeds** — it produces an rlib, and an
  rlib is never linked. The failure arrives later as
  `LNK1181: cannot open input file 'sqlite3.lib'`, and only on a machine that
  has no system SQLite. On this machine it does not, so this is also the direct
  evidence for the acceptance criterion.
- Adding `tokio` to `[workspace.dependencies]` fails
  `no_async_runtime_has_arrived`. Confirmed the test is not vacuously green: the
  same line-based reader finds `rusqlite` in the same section.
- Removing the in-transaction version re-read in `apply_one` fails
  `a_migration_another_process_already_ran_is_not_run_a_second_time` with
  `table records already exists`, and **does not** fail the cross-process test.
  With the journal-mode wait in place the children are serialised past that
  window. Both tests carry a comment saying so; the cross-process test says what
  it does not cover.
- Earlier in this branch: `P1-T007`'s `additionalProperties: false` bug (checked
  inside the `properties` lookup, so a closed object with no `properties` allowed
  every key) and the `issue` → `issue_id` repair-contract gap.

**The file name that is not valid Unicode, and how it got tested.** The previous
entry in this file recorded it as a gap ("cannot be constructed on Windows at
all"). It can be: `OsString::from_wide(&[0xD800])` is an unpaired surrogate, Rust's
`OsString` on Windows is WTF-8 so it holds one, and NTFS does not forbid it. It is
now tested, and it is the test that makes the sort mutation visible, because a
name that cannot be rendered is the *only* case where sorting by name and sorting
by rendered text disagree: an unpaired surrogate and `\u{E000}` sort one way by
bytes and the other way as text. Both orders were checked with a standalone probe
first. macOS is excluded from the test, and the reason — APFS validating a file
name as UTF-8 — is written down as a belief rather than a fact, because verifying
it needs a Mac.

## Accepted work on this branch

- `4f2d75d` P0-T009 — foundational ADRs, plus `FROZEN_SEMANTICS.md`.
- `4ce2ce6` P1-T001 — declared crate boundaries, mechanically enforced by
  `sure_testkit::workspace` and `sure_testkit::integrations`.
- `6f63801` P1-T002 — `variants!` `ALL` lists and the pinned wire contract.
- `P1-T003` — `crates/sure-core/src/config/`: the `sure.yaml` model, loader,
  diagnostics and redaction, plus `docs/architecture/CONFIG_REFERENCE.md` and
  `docs/adr/0011-project-configuration-is-a-request.md`.
- `b6a14b4` P1-T004 — OS-native data paths and the outside-the-project rule.
- `ad05ee1` P1-T006 — diagnostics as records, and redaction.
- `P1-T007` — `crates/sure-protocol/`: the schema validator, the document
  registry, the event envelope, 27 conformance/round-trip tests and
  `docs/architecture/PROTOCOL.md`.
- `P1-T005` — `crates/sure-core/src/store/`, the concurrency and packaging tests,
  and §The store in `STORAGE_AND_DATA_PATHS.md`.
- `P1-T008` — `crates/sure-cli/`, `docs/architecture/CLI.md`, and the `PROTOCOL.md`
  amendment that keeps "everything SURE writes is one of seven documents"
  literally true.
- `ec8c293` P1-T009 — `sure doctor`, `crates/sure-core/src/doctor.rs` and
  `crates/sure-cli/src/doctor.rs`.
- `8892e48` P1-T010 — `crates/sure-protocol/src/handshake.rs`, `sure protocol
  --speaks`, and §The handshake in `PROTOCOL.md`.
- `b6ca862` P1-T011 — `crates/sure-core/src/config/authority.rs`, `load_file`,
  and the rewritten `CONFIG_AUTHORITY.md`. **This closed P1.**
- `82f3cf7` P2-T001 — `crates/sure-core/src/scan/`, `tests/scan_project.rs`, and
  the new `docs/architecture/PROJECT_DISCOVERY.md`. **This opened P2.**
- P2-T002 — `crates/sure-core/src/fingerprint/` (the `git` and
  `digest` modules), `tests/fingerprint_git.rs`, the new
  `docs/architecture/FINGERPRINTING.md`, and the `scan/ignore.rs` `left_out`
  extraction. The commit hash is in `progress/state.json`'s `P2-T002` note and in
  `git log --oneline -n 4`.
- `58b3793`, `c735a2f`, `9f13f0d` — three follow-up commits on the *accepted*
  `P2-T002`, none of them a new task. `58b3793` and `c735a2f` are the CI fixes
  (the first was a guess and failed; the second read the log and worked), and
  `c735a2f` also refuses a path that climbs out of the project. `9f13f0d` stops
  a repository making Git run a program or hang a check. **`P2-T002`'s acceptance
  stands over all four**: none of them changes a verdict for a project that is
  not hostile, and the acceptance run's own red CI is recorded above rather than
  quietly re-run.
- `5705444` — a fifth follow-up on the *accepted* `P2-T002`, and the only one
  that changes a verdict for a **non-hostile** project: a repository that names a
  filter is now refused where it used to be fingerprinted. That is the whole
  point of it (see "What the filter hardening added" above), and it is a real
  behaviour change rather than a hardening nobody can observe. `P2-T002`'s
  acceptance still stands — its two criteria are about a project that is not
  hostile — but this is the commit to look at if a legitimate project starts
  being refused.
- `7ce90bf` **`P2-T003`** — `crates/sure-core/src/fingerprint/{content,choose,
  read}.rs`, `tests/fingerprint_content.rs`, and the rewritten two-kinds half of
  `docs/architecture/FINGERPRINTING.md`. The `read.rs` extraction is the one part
  of it that is a refactor rather than new behaviour, and it is the part with the
  least new test cover — deliberately, because its cover is the Git kind's tests,
  which did not change and did not need to.
- `68e51d8` **`P2-T004`** — `crates/sure-core/src/discover/{mod,read,node}.rs`,
  `tests/discover_node.rs` (33 tests, the new 19th test binary), and the new
  `docs/architecture/ECOSYSTEM_DISCOVERY.md`. The module is a reader: it executes
  none of the scripts it reports, which is the property it is shaped around.
- `e10f620` **`P2-T005`** — `crates/sure-core/src/discover/python.rs`,
  `tests/discover_python.rs` (31 tests, the new **20th** test binary), the
  Python half of `docs/architecture/ECOSYSTEM_DISCOVERY.md`, the promotion of
  `toml` from a test-support dependency to a real one, and small extensions to
  `mod.rs` (`Ecosystem::Python`, `Findings::Python`), `read.rs` (`read_text_file`
  and the four `toml`-conversion tests) and `scan/ignore.rs` (`.tox`, `.nox`,
  `.eggs` as vendored). **A second ecosystem is the point of it**: the enum that
  refuses to let "not there" arrive as "there but unreadable" survived being
  written a second time, and the one-budget-serves-both fact is recorded rather
  than discovered.
- `9c931d0` P2-T006 — `crates/sure-core/src/discover/rust.rs` (2885 lines, 30 unit
  tests), `tests/discover_rust.rs` (24 tests, the new **21st** test binary),
  `src/discover/pattern.rs` (the member-pattern expansion lifted out of
  `node.rs`), the Rust half of `docs/architecture/ECOSYSTEM_DISCOVERY.md` with
  five new gaps, and small extensions to `mod.rs` (`Ecosystem::Rust`,
  `Findings::Rust`, `MemberManifest` lifted) and `node.rs` (the re-export that
  keeps its old names). **The third ecosystem closes the trio, and it is the one
  that put the readers under real pressure**: `Cargo.toml` has a shape neither of
  the other two has, in that its two top-level tables are siblings either of which
  can be absent, so the "not there is never there-but-unreadable" rule could not
  simply be copied — `Manifest` and `PackageSection` had to be split, and the
  commands had to be gated on the document rather than on the package. Both splits
  are pinned by mutations that revert them.
- `0a577ca` — a defect fix, **not a task**, landed just before `P2-T004`'s
  implementation commit and found while verifying it. Five test helpers cleared a
  scratch directory with `let _ = remove_dir_all` and then treated the path as
  fresh; on Windows that deletion can fail and the discarded error produced a
  **false report** — `store_concurrency` announced 200 records where 100 had been
  written, about a file that was never cleared. One instance is proven, four are
  not, and the section above says which is which. Nothing about the product's
  behaviour changes; every file it touches is test code.

## Next concrete action

1. **`P2-T010` is accepted and its chain is complete.** `4746c48` is the
   implementation, run `34869888350`, all five jobs green, **907 / 909 / 910**
   with 0 failed and 1 ignored over **37** result lines = **27 parents + 10
   children** — read, and attributed by binary name, in "Reading run
   `34869888350`". The acceptance is the commit carrying this file, and **its run
   is read in the session that took it rather than committed** — the stopping
   rule at the top of this file.
2. **The next task is a real choice, and the list is 12 long.**
   `P2-T008`, `P2-T009`, `P2-T012`, `P3-T001`, `P4-T007`, `P4-T008`, `P6-T001`,
   `P6-T005`, `P6-T007`, `P8-T001`, `P12-T008`, `P13-T001`. Phase `P2` is **8 of
   12**; `P2-T008`, `P2-T009` and `P2-T012` would finish it. They extend a
   component graph that is now three commits old and **still has no consumer** —
   `ComponentGraph` is produced by `P2-T007` and read by nothing — so the argument
   for them is coherence of a phase rather than a pull from anything downstream.
   `P4-T007` and `P6-T005` remain **unread by any session**, and `P6-T007` and
   `P8-T001` have been on the list since before this file was written. **Read the
   task entry before choosing**; do not choose from this paragraph.
3. **`P2-T007` is accepted, its two commits are pushed and read.**
   `586d3a3` the implementation in run `34864498113` — Windows **871** / macOS
   **873** / Ubuntu **874**; `435181f` the run record; `0907acf` the acceptance.
   `node scripts/taskctl.mjs status` now reads `{ accepted: 28, queued: 138 }`
   with **nothing `in_progress`**, so the next session may start any `READY` task
   without adopting an orphan.
4. **The store now holds six rows that no user wrote, and that is the first item
   for whoever next touches `--goal` or the mutation harness.** They are listed in
   "The mutation run wrote six rows into the real store" above, with the reason
   they exist and the one-line statement that removes them. **They are left in
   place deliberately**, because removing rows from a history file with no backup
   is not reversible and is the owner's decision, not this session's. Two
   consequences: a mutation that deletes the empty-goal refusal **will do it
   again**, and any local experiment that runs the `cli_contract` binary writes to
   the developer's real store until a caller can choose where SURE keeps its files
   — which is the same gap `docs/architecture/CLI.md` records as missing test
   coverage.
   **Keep the ordering discipline**: push each task's commits, read that run, and
   only then start the next acceptance. The cost of not doing it is written down
   four times in this file now. **`P2-T006` paid for it in a new currency**: the
   per-target counts read out of a CI log by proximity were wrong three times for
   `P2-T005`, so the whole-step count is cross-checked by a **multiset**
   comparison. **`P2-T007`** compared that multiset against the previous run's
   position by position. **`P2-T010` added the third increment, and it is the one
   to carry forward: compare by binary name, and print the number of matches.**
   Two attempts here reported a well-formed table of zeros — the first because
   `Running` in a GitHub log is followed by an ANSI escape rather than a space,
   the second because a name was carried across a line the pattern had missed and
   labelled another binary's count. Both looked exactly like "no binary changed".
   `target/tmp/bincounts.py` exists so the next session does not rediscover it.
5. **`project_fingerprint` now has one caller, and it is not a check.**
   `sure check --goal` fingerprints the project to bind a recorded goal to a
   state; nothing constructs an `Authority`, nothing runs the check pipeline, and
   nothing compares a goal against a project. So `FINGERPRINTING.md`'s coverage
   rule and the dispatch rule are still properties of the modules and their
   tests, verified, and **not yet properties of a `sure check`** — the one
   invocation that reaches the fingerprinter reaches it for a goal, and reports
   the kind and the digest rather than checking anything. The documentation says
   so in as many words; do not let a later summary of this branch imply
   otherwise.
5. **`project_fingerprint` now has one caller, and it is not a check.**
   `sure check --goal` fingerprints the project to bind a recorded goal to a
   state; nothing constructs an `Authority`, nothing runs the check pipeline, and
   nothing compares a goal against a project. So `FINGERPRINTING.md`'s coverage
   rule and the dispatch rule are still properties of the modules and their
   tests, verified, and **not yet properties of a `sure check`** — the one
   invocation that reaches the fingerprinter reaches it for a goal, and reports
   the kind and the digest rather than checking anything. The documentation says
   so in as many words; do not let a later summary of this branch imply
   otherwise.

**What `P2-T002` left for later, and what `P2-T003` then did with it.**
`P2-T002` left the Git fingerprint asked for explicitly, by a caller that had
already decided the project is in a repository, with nothing making that decision
for it. `P2-T003` wrote the decision — `project_fingerprint` in `choose.rs` — and
that is as far as it goes: **nothing calls `project_fingerprint` yet either.**
So `FINGERPRINTING.md`'s coverage rule — *a file is part of the fingerprint if
and only if a check could read it* — and the dispatch rule above it are, like
`PROJECT_DISCOVERY.md`'s guarantees, properties of the modules and of their tests
until the check pipeline is the first real consumer.

The `#[cfg(unix)]` tests have still not run on this machine and never will; they
run on the macOS and Linux CI jobs. Four were read out of run `34839532984` by
name rather than inferred from the job's colour, and `P2-T003` added two more —
both read out of run `34845282962` **on both Unix jobs** by name, in the section
above. See the top of this file.

`taskctl accept` takes `--note`, not `--evidence`; `--evidence` is silently
ignored, which is how the earliest tasks came to record an empty note.

## Environment notes for the next session

- **Run `cargo fmt --all` before the gate set, not after.** New files written by
  hand are not rustfmt-shaped (let-else bodies, long `assert!` messages) and
  `--check` fails on them. `P1-T007`'s commit was blocked once by this.
- **Do not read or write repository sources with Python's default encoding.** It
  is `gbk` on this machine, and a source file with a `—` in it fails with
  `UnicodeDecodeError: 'gbk' codec can't decode byte 0x94`. Pass
  `encoding='utf-8'` and `newline='\n'`.
- **Parse the command line with `clap::Parser::try_parse`, never `parse`.**
  `parse` calls `process::exit` itself, which would put one row of SURE's
  documented status table in a library's hands. `status_of` maps clap's exit code
  onto `report::exit`; `--help` and `--version` are clap's 0 and become SURE's 0.
- **`arg_required_else_help = true` makes a bare `sure` exit 2, not 0.** That is
  deliberate: it did nothing, so status 0 would let a script that invoked the
  wrong thing read it as a clean run.
- **`env!("CARGO_BIN_EXE_sure")` gives an integration test the built binary**,
  and an integration test may use its package's `[dependencies]` — which is why
  `tests/cli_contract.rs` can `serde_json::from_str` the frame without a
  dev-dependency of its own.
- Clippy's `unnecessary_map_on_constructor` is enforced by `-D warnings`:
  `Some(x).map(Some)` is an error, `Some(Some(x))` is not.
- **A mutation can leave the responsible test green because the test and the
  code share a source of truth.** Changing what `Report::command()` returns does
  not fail `report.rs`'s own frame test, which compares the frame against
  `report.command()`; the integration test that reads a real process's stdout is
  what catches it. When a test asserts `f(x) == g(x)` and both sides call the
  same function, check which test is actually load-bearing before trusting it.
- **A mutation anchor must be copied out of the file, not out of memory.**
  `rustfmt` reflows arms and arguments, so the text a mutation replaces is often
  not the text that was written. A script that finds its anchor zero times
  reports SKIP and has tested nothing; both mutation scripts print SKIP loudly
  for that reason. `P1-T010`'s first CLI mutation hit exactly this.
- **A mutation inside a test's stated range is evidence about the test; one
  outside it is not.** `P1-T010`'s first reader mutation accepted a version
  above the range the test loops over, and escaped — correctly. Writing the
  mutation to land between two values the test covers is what turned it into
  evidence.
- **`clap` answers a typed argument's parse failure with exit 2**, which is
  `sure protocol --speaks latest` → "you typed it wrong" rather than "SURE
  cannot talk to that". `an_unknown_command_or_flag_is_a_wrong_command_line`
  pins it; the distinction matters because 3 reads as a version that exists.
- **The PowerShell here-string cannot carry a git commit message.** Use the Bash
  tool for `git commit -F - <<'EOF'`; PowerShell rejects the redirection.
  (`@'…'@` works in PowerShell but the closing `'@` must be at column 0.)
- **Every integration test file needs
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the
  top.** The workspace lints are `warn` but the gate runs `-D warnings`, so a new
  test file fails clippy until it carries the opt-out. `cargo fmt --all` will
  place it correctly if the file starts with it.
- **A directory junction reports `is_symlink() == true, is_dir() == false`** from
  `DirEntry::file_type()`. Checked with a probe, not assumed. This is why the
  walk matches the symlink arm first: put it after `is_dir()`/`is_file()` and
  every junction on Windows falls into the `Some(_)` arm as a `SpecialFile`, and
  the "never followed" rule still *looks* enforced because nothing is followed.
- **`mklink /J` (junction, no admin needed) parses forward slashes in its
  arguments as its own switches.** Build the argument vector with backslashes —
  `r"target\tmp\..."`, not `"target/tmp/..."` — or it fails with a usage error in
  the console's own code page. It is also invoked through `cmd /C`, so a Rust
  string you pass it must be a raw string or the `\t` is a tab.
- **Win32 strips trailing spaces and dots from the last path component before the
  file is created.** A fixture asking for `"padded "` produces `padded` on Windows
  and `padded ` on Unix; `a_trailing_space_is_not_a_character_on_windows` is the
  test that pins the difference, so the omission elsewhere is a fact with a test
  rather than a hole. An interior or leading space is kept.
- **A file name that is not valid Unicode *is* constructible on Windows**:
  `std::os::windows::ffi::OsStringExt::from_wide(&[0xD800])` gives an unpaired
  surrogate, NTFS accepts it, and `to_string_lossy` renders it U+FFFD. On Unix it
  is `OsStringExt::from_vec(vec![0xED, 0xA0, 0x80])`. APFS is believed to refuse
  it; that belief is recorded in `PROJECT_DISCOVERY.md` rather than relied on
  silently.
- **Sorting by `to_string_lossy()` and sorting by `OsString` agree for every pair
  of names that both render faithfully, and differ for exactly one pair**: a name
  that cannot be rendered (lossy U+FFFD) against a name whose code points are all
  above U+FFFD. `\u{D800}` vs `\u{E000}` sort one way by bytes and the other way
  as text. A mutation to the text sort therefore escapes every test that uses
  ordinary names — which is why the invalid-name test is the one that has to
  exist.
- **`git status --relative` is unusable on this machine's Git (2.55.0.windows.3):
  for a repository with changes it prints nothing at all and exits 0.** The first
  version of `Git::STATUS_ARGUMENTS` used it, and the failure mode is the worst
  one available — an empty change list, which is indistinguishable from a clean
  tree, so a dirty project would have fingerprinted as clean and every check
  keyed to it would have gone green. It was caught by
  `a_dirty_project_fingerprints_differently_from_a_clean_one`, not by reading the
  flag list. The replacement is `-- .` plus a separate `git rev-parse
  --show-prefix`, stripped with `Path::strip_prefix` (a string comparison would
  confuse a project in `app/` with a sibling `app-old/`). The flag is now
  forbidden by a comment in the module and by a mutation in `mutate6.py`;
  `FINGERPRINTING.md` states the prohibition. **No other Git flag is used here
  without having been run against a real repository first.**
- **A nested `git init` inside a fixture is reported as one untracked directory**
  — `? inner/` — even under `--untracked-files=all`, verified by hand. That is
  what makes the walk in `Reader::tree` reachable and what a test had to
  construct; without it the whole nested-checkout path is dead code that looks
  alive.
- **This machine has no Rust toolchain available for the Unix tests, and that is
  CI's job rather than a local one.** Windows cannot create a symbolic link
  without Developer Mode or administrator rights (both probed, both absent), and
  the WSL Ubuntu that is installed here has Git 2.53.0 but **no `cargo`**.
  Installing one would be a 1–2 GB unilateral change to the owner's machine, and
  `.github/workflows/ci.yml` already runs `cargo test --workspace` on
  `ubuntu-latest` and `macos-latest` — which `RUST_DESIGN.md` names as the
  designed verification path for `#[cfg(unix)]` behaviour. As of run
  `34839532984` that path is **observed working**, not merely intended: the three
  `#[cfg(unix)]` fingerprint tests from `P2-T002` passed there, and the two
  case-rule tests in `paths/compare.rs` ran one per platform as designed.
- **`mkfifo` is how a test puts a pipe in a working tree**, and it is an external
  program rather than a `libc` call because the workspace is
  `unsafe_code = "forbid"`. It exists on both CI Unix images. A test that cannot
  create its fixture must **fail, not skip** — a skip that reads as a pass is the
  thing this repository keeps finding.
- **The two facts about Git and pipes, measured in WSL with Git 2.53.0 and worth
  not re-deriving.** A tracked path whose working-tree entry is replaced by a
  pipe **is** reported, as an ordinary modified file (`1 .M N... 100644 100644
  100644 …`), so it reaches `Reader::read`. An *untracked* pipe is **not** in
  `git status --porcelain=v2 -uall` output at all. Anything concluded from a
  pipe's absence from a fingerprint is concluded about Git first.
- **A throwaway `rustc -D warnings` file is the only local check available for a
  `#[cfg(unix)]` body**, and it is worth using: it catches a type or lint error
  (it was used for `Result::is_ok_and(ExitStatus::success)` and for an un-joined
  `thread::spawn`, and confirmed `JoinHandle` is not `#[must_use]`). It cannot
  catch a wrong *expectation* — that is what the runner is for. Compile the
  construct, not the code, and do not let it stand in for the run.
- **A "reads no file contents" guarantee can only be tested against the source.**
  `scanning_the_repository_does_not_open_any_file` greps the four `scan/*.rs`
  files for `File::open`, `fs::read(`, `fs::read_to_string`, `read_to_end`,
  `BufReader`, `read_link` and `canonicalize`. No run of the code demonstrates
  that a file was never opened. Adding `let _ = fs::read(...)` to the walk makes
  it fail, which was checked by mutation.
- **Rank a closed enum with a full `match` returning a number, not with a
  `bool` predicate.** `P1-T011`'s first `resolve` took
  `stronger: impl Fn(T) -> bool` (e.g. `|mode| mode == Standard`), which silently
  ranks every variant the predicate does not name as *weaker* — including
  `ProtectionMode::Custom`, a variant this release cannot produce but which
  exists in the enum. A `match` forces the question to be answered when a variant
  is added, and lets the unreachable ones be ranked in the safe direction with a
  comment saying so. **A predicate that answers `false` for an unhandled case is
  a default, and defaults are where false greens live.**
- **`execution.allow_network: true` and `allow_dependency_install: true` are
  `Contradiction`s unless `execution.mode` is non-`inspect_only`.** Any test
  fixture that sets either boolean must also set `mode: host_confirmed`, or it
  fails at parse time with a message about a setting that "could never take
  effect".
- Write repository files with LF endings. `core.autocrlf=true` plus
  `.gitattributes` (`* text=auto eol=lf`) means a Python `write_text` on Windows
  leaves CRLF in the working tree that shows as a phantom ` M` until
  `git add -A` re-hashes it. Write with `newline='\n'` or `write_bytes`.
- **`.gitattributes` and `git ls-files --eol` are the authority on line
  endings.** `grep -c $'\r'` gives a false positive on some files and a false
  negative on others; `store_packaging.rs` was `w/crlf` with zero `\r` bytes.
  Use `git ls-files --eol`.
- **`rusqlite` is a `dev-dependency` of `sure-core` as well as a real one**,
  which looks like a mistake and is not: `tests/store_concurrency.rs` holds the
  write lock from outside the store, and the lock is SQLite's, so the test has to
  take it with SQLite. `[dev-dependencies]` do not appear in
  `normal_edges()`, so this does not affect the crate-boundary test.
- **A spawned test child must be given `--exact <name> --ignored`, not a bare
  filter.** Without `--exact` a filter that is a prefix of another test's name
  runs that one too, and the child recurses into the parent's tests.
- **A cross-process contention test needs a barrier or it tests nothing.** See
  the two-bugs section above. The pattern that worked is a wall-clock moment
  passed in an environment variable and a spin loop in the child; Windows sleeps
  in ~15 ms steps, which is too coarse.
- The child's environment variable holds a path, a count and a time,
  newline-separated. Any other separator can appear in a path.
- `toml` 1.x parses a *document* into `toml::Table`, not `toml::Value`;
  `Value`'s `FromStr` reads a single value and fails on the second key.
- `variants!` `ALL` is a `&'static [Self]` slice, so iterate with
  `for &x in T::ALL` and use `.iter().copied()` where a value is needed. The
  macro is `#[macro_export]`ed, so `sure-core` reaches it as
  `sure_domain::variants::variants`.
- `serde_yaml_ng` error text is load-bearing for `config`'s diagnostics: it
  prepends a `parent.path: ` prefix, quotes field names in backticks and values
  in double quotes, and reports a repeated key with the *parent* path. The three
  shapes are pinned by `serde_error_shapes_are_what_this_classifier_expects`.
- YAML 1.2 (what `serde_yaml_ng` implements) does **not** read `yes`, `no`, `on`
  or `off` as booleans. `checks.existing_tests: yes` is a string, and the
  classifier turns that into "use one of: true, false".
- Repository test fixtures that need a writable scratch directory belong under
  `target/tmp/` (already git-ignored, same volume as the checkout). See
  `crates/sure-core/tests/config_loading.rs`.
- **…and a fixture under `target/tmp/` is inside this repository's working tree,
  which is a fact about the fixture and not about the test.** Any test whose case
  is "there is no repository here" needs `std::env::temp_dir()` instead. The
  `P2-T003` test named for that case was under `target/` and exercised the other
  branch for a whole acceptance run without anything going red; the mutation that
  changed the branch was MISSED, which is the only reason it was found.
- **Assert the premise with the tool that is not under test.** `git_prefix()` in
  `tests/fingerprint_content.rs` runs `git rev-parse --show-prefix` and asserts
  what it printed, so a test that intends to be "inside somebody else's
  repository" fails rather than quietly becoming a different test. Its first
  version compared `Some("inner/")` against `"inner/\n"` and the *reading* was
  wrong while the assertion was right — which is the good direction to be wrong
  in.
- **`Git::with_program(name)` is the only way to reach `GitUnavailable`**, and it
  needs the function that would use it to be `pub(crate)`: an integration test
  cannot call it. `choose.rs`'s `fingerprint_with` is `pub(crate)` for exactly
  this and its test lives in the module as a result. The rule generalises — when
  an outcome is decided by the *machine* rather than by the fixture, the test
  cannot be an integration test.
- Clippy's `derivable_impls` and `result_large_err` are enforced by
  `-D warnings`. `ConfigError` boxes its `ErrorKind` for the second reason; the
  non-derivable `Default` impls (`ExecutionConfig`, `ChecksConfig`) carry their
  reason in a comment.
- **An `f64` read out of JSON has no total order, so a type holding one must not
  derive `Eq`.** `ViolationKind::BelowMinimum` is why `Violation`,
  `ViolationKind` and `EnvelopeError` are `PartialEq` only; each carries a
  comment saying so. `assert_eq!` needs only `PartialEq`.
- **`DocumentKind::ALL` is a free const** (`sure_protocol::documents::ALL`), not
  an associated one, and `DocumentKind::FixtureExpectation` is deliberately not
  storable — `RecordKind::is_storable` is the predicate and `Store::append`
  refuses it with `StoreError::NotStorable`.
- The event schema has `additionalProperties: false`. A test fixture that needs
  to carry its own data must put it under `payload`, which is the only open
  object.
- **A raw string in Rust ends at the first `"#`.** `r#"{"$ref":"#/x"}"#` does
  not compile; write `r##"…"##`.
- Cross-target clippy needs the target installed. `x86_64-pc-windows-msvc` and
  `x86_64-unknown-linux-gnu` are; `aarch64-apple-darwin` is not, and asking for
  it fails with `E0463`.
- **MSVC builds without `cl.exe` on `PATH`.** The `cc` crate locates Visual
  Studio itself, so `rusqlite`'s `bundled` feature compiles SQLite 3.50.2 here
  with no environment setup. This was checked by building a probe, not argued.
- `jsonschema` was evaluated for `P1-T007` and **rejected**: it pulls
  `reqwest` + `rustls/aws-lc-rs` and roughly fifty transitive crates, which is
  the wrong shape for a local-first tool. Do not reintroduce it without reading
  `docs/architecture/PROTOCOL.md` §The validator, which records the two
  invariants the hand-written replacement must keep (an unsupported keyword is
  an error, not a skip; the schema is not treated as an annotation).

## External blockers

None. No task has been marked `block-external`. No credential or authorization
outside this machine has been needed yet.

### A second review notification, this one acknowledged by inspecting the file

At 2026-09-14T13:47Z, during the `P2-T004` work, a push security review notified

> Push security review found: Path Traversal / Symlink TOCTOU in crates/sure-core/src/discover/read.rs

**The finding text was empty again** — the body was the same harness telemetry
payload as the notification below (`[claude-code:unrecognized_model]`, a session
-title query), not a statement about the code. So there was again nothing to act
on. **But this one named a file written in this session and a risk class that
could plausibly be real**, so it was not merely filed: `read.rs` was read, and
the question "can a project make SURE read a file outside itself?" was answered
from the code rather than from the notification.

**What the inspection found, in the order that matters.** Containment is by
construction and not by a check: a path a manifest names goes through
`contained_relative`, which returns only `Component::Normal`; each component is
then resolved against the **walk's own records** (`Tree::is_directory`,
`Tree::child_directories`) and never by joining onto the filesystem; and
`read_json` takes the `Probe` the walk produced rather than a path, so what is
read and what was found cannot come from two different walks. A directory link
inside the project therefore cannot redirect a workspace pattern, because the
walk never followed it and the tree does not record it as a directory.

**One window is real and was not recorded anywhere.** `read_text` calls
`File::open(root.join(entry))`, and an open follows whatever is at that path
*now* — so a project that swaps `manifest` for a link pointing outside, between
the walk and the read, is read through. It is now `ECOSYSTEM_DISCOVERY.md`'s gap
6, with what bounds it (that read is the one place the filesystem is consulted
twice), what it is not (an **integrity** problem, not a disclosure — the content
lands in a report read by the person who can already open the target, and it
changes nothing that is executed), and why it is left open (`O_NOFOLLOW` and
`FILE_FLAG_OPEN_REPARSE_POINT` are the fixes; the Windows flag needs `unsafe`,
which this workspace forbids). `FINGERPRINTING.md` gap 5 is the same window for
the fingerprint's reader.

**So the honest summary is: a content-free notification, an inspection that
found no traversal, and one real race recorded as a gap rather than fixed.** The
gap is the owner's to weigh, not this session's to close — and it is written down
so that the next person does not have to rediscover it from an empty payload.

### One unactioned notification, recorded rather than dropped

At 2026-09-14T11:05Z a background task notification arrived reading

> Push security review found: issue in crates/sure-core/src/config/mod.rs

and the reminder body that followed it, under "address or acknowledge the
findings below", contained no finding — the text there was
`[claude-code:unrecognized_model] {"model":"deepseek-flash","query_source":"generate_session_title"}`,
which is a harness telemetry payload about generating a session title and not a
statement about the code. **There was nothing to act on and nothing to dismiss:**
the file was named and the reason was not.

It is recorded here rather than silently dropped because "reviewed, nothing
found" and "never actually reviewed" are the two states this project keeps apart
everywhere else, and a notification that arrives with an empty payload is the
second one wearing the first one's clothes. `crates/sure-core/src/config/mod.rs`
was read at the time and holds nothing that looks like a finding: it validates
endpoint URLs, refuses credentials embedded in a URL as userinfo, and
deliberately does not repeat a credential's value in an error message. That is a
reader's impression, not a review result, and it is written here as one.

**If a later session finds a real security review waiting, treat this paragraph
as the acknowledgement and act on the actual finding.** Nothing depends on the
absent text.

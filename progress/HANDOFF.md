# Autonomous handoff

Last updated: 2026-09-14
Branch: `claude/v0.1-autonomous`
Progress: 22 / 166 tasks accepted. **Phase P0 complete (9/9), phase P1 complete
(11/11), phase P2 in progress (2/…).** `P2-T002` is accepted, and its acceptance
is recorded in `progress/state.json` **in the same commit as this file** — so
`git log -1 --stat` is the check. If that commit's subject does not name
`P2-T002`, the acceptance is not recorded and the task is not done.

A correction to the two entries before this one: each said `progress/state.json`
records the acceptance "in the commit immediately after the one carrying this
file". Neither did — in `07e20be` and its predecessor the handoff and the
acceptance landed together, so the sentence described a procedure that was not
the one followed. Stated here because a handoff that is wrong about how to
verify it is worse than one that says nothing.

**Since that acceptance, `P2-T002` has had three follow-up commits — and the
first two of them exist because CI had been red since the bootstrap commit and
nobody had read it.** That is the most important thing in this file: the local
gate set on this machine cannot see platform-gated code, `P2-T002` was accepted
while three CI jobs were failing, and the acceptance was sound only by luck. The
account is in "Continuous integration" below, and the rule that came out of it is
**a push is not finished until its run has been read**.

Primary development host: Windows 11 x64 / native MSVC.

Canonical remote: `https://github.com/lichman0405/SURE.git`
Autonomous branch: `claude/v0.1-autonomous`

## Exact current state

`node scripts/taskctl.mjs status` reports:

```
Project: SURE | status: in_progress | phase: P2
{ accepted: 22, queued: 144 }
READY: P2-T003, P2-T004, P2-T005, P2-T006, P2-T010, P3-T001, P6-T001, P6-T007,
      P8-T001, P12-T008, P13-T001
```

**`P2-T002` (Git project fingerprint) is the work of this session.** All of it is
on `claude/v0.1-autonomous` and green: `sure_core::fingerprint` with the `git`
and `digest` modules behind it, 37 new unit tests, 34 new integration tests in
`crates/sure-core/tests/fingerprint_git.rs` (3 more are `#[cfg(unix)]` and first
run on CI), and the new `docs/architecture/FINGERPRINTING.md`. `P2-T003` (the
non-Git fingerprint) is the next ready task, and `fingerprint/mod.rs`
deliberately offers no function that picks between the two kinds — choosing by
looking at the project is the decision `P2-T003` owns, and it is the one that can
be wrong (a project inside somebody else's repository).

**The thing this session could not verify locally is now verified, and the
evidence is named rather than assumed.** `a_link_is_recorded_by_its_target_and_not_by_what_it_points_at`,
`a_change_behind_an_unchanged_link_is_not_a_change` and
`a_change_to_a_file_sure_cannot_read_has_no_fingerprint` are `#[cfg(unix)]`, and
the body of the second was **rewritten this session without ever having run on
this machine** — it previously asserted almost nothing (see the mutation section
below). Windows cannot create a symbolic link without Developer Mode or
administrator rights, and both were probed and are absent. WSL Ubuntu exists here
with Git 2.53.0 but no Rust toolchain.

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
the repository: the 15 became 16, and the doc-test zeros became a one.
`target/tmp/count_tests.py` **used to skip the `Doc-tests` sections entirely**,
which is why the figure it printed and the figure in the handoff disagreed by
one until both were changed together. It now counts them and prints them
separately.

**Counting `#[test]` attributes does not reproduce these figures**: `sure-domain`'s
`variants!` macro generates tests that no attribute names, and it undercounts the
suite by about twenty. Take the numbers from a run.

### Count the parent lines, not the `test result:` lines

**The raw number of `test result: ok` lines overstates this suite.** A workspace
run prints **28** of them for **429** tests, because `store_concurrency` spawns
**ten** child processes (4 writers + 6 openers) and each child prints its own
`test result: ok. 1 passed; … 6 filtered out` into the parent's stdout. `--quiet`
does not suppress that summary line — libtest's `--quiet` drops the
`running N tests` line and the per-test lines and still prints the summary. The
comment in `spawn_child` said otherwise and has been corrected.

This matters for the record, not just for tidiness: the figure written into
`P1-T005`'s acceptance note (**408 passed**) was a raw sum of those lines and is
therefore **inflated by the child lines**. The true parent-only figure at
`P1-T005` was 396 passed / 1 ignored, i.e. 397 tests; `P1-T008` adds `sure-cli`'s
33, which is the 429 above. Nothing regressed — the earlier number was counted
wrong.

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
the four `#[cfg(unix)]` bodies, three of which predate this session.

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
five tests added this session exist because a mutation survived (the section
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

## Adversarial (mutation) verifications in this session

Each was reverted after confirming the check fires.

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
- P2-T002 (this session) — `crates/sure-core/src/fingerprint/` (the `git` and
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

## Next concrete action

1. `node scripts/taskctl.mjs start P2-T003` — the non-Git/content fingerprint.
   **`fingerprint/mod.rs` deliberately offers no function that chooses between
   the two kinds**, and that is the decision this task owns. `FingerprintKind`,
   `ProjectFingerprint::content` and the `Content` digest domain already exist
   and are unused-by-design; what `P2-T003` has to answer is *when* a project is
   fingerprinted by content rather than by Git, and the case that can be answered
   wrongly is a project that sits **inside somebody else's repository** — a
   directory under a checkout that is not its own. Fingerprinting that by the
   outer Git would make its evidence go stale whenever the outer repository's
   unrelated work changed, and it would report a `GitState` for a repository the
   project is not in. The other half of the task is the non-Git case proper: no
   `git` on the path, or a directory that is not a working tree at all.
   `Git::fingerprint` already refuses a non-absolute root and reports
   `GitUnavailable` distinctly from `GitFailed`, so which of those two refusals
   `P2-T003` turns into a content fingerprint and which stays an error is a
   decision worth stating in `FINGERPRINTING.md` rather than inferring.
2. Then `P2-T004`–`P2-T006` (JS/TS, Python and Rust discovery), then `P2-T010`
   (ProjectIntent ingestion from an explicit goal/spec), which `Config` already
   carries a slot for.

**What `P2-T002` deliberately left for later.** The Git fingerprint is asked for
explicitly, by a caller that has already decided the project is in a repository;
nothing in this release makes that decision for it, and nothing calls
`Git::fingerprint` yet. So `FINGERPRINTING.md`'s coverage rule — *a file is part
of the fingerprint if and only if a check could read it* — is, like
`PROJECT_DISCOVERY.md`'s guarantees, a property of the module and of its tests
until the check pipeline is the first real consumer. The `#[cfg(unix)]` tests
have still not run on this machine and never will; they run on the macOS and
Linux CI jobs, and four of them were read out of run `34839532984` by name rather
than inferred from the job's colour. See the top of this file.

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

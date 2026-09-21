# FINAL_REPORT.md

SURE — Software Understanding & Reality Evaluation — is a project truth and
checking layer for AI-built software. It is pointed at a directory, reads what
that directory declares about itself, plans the checks that would establish
whether the project is what it says it is, and refuses to call the project clean
when the checks it planned did not run. It is not a coding-agent orchestrator and
it does not claim to have verified anything it did not measure.

This report is the deliverable the brief names at `MASTER_PROMPT.md:230` — *"a
ready branch/PR plus `FINAL_REPORT.md`"* — and its acceptance clause is
`MASTER_PROMPT.md:328`: *"`FINAL_REPORT.md` exists with exact
commands/results/limitations."* Every reading below was taken at the commit
named, and every place this report could not measure something says so rather
than filling the gap.

## 1. What this is, and the exact commit it describes

**The readings in this report were taken at
`42812fc15e5465fb8280ab53a4f77e11ff03b26e`** — the tree the seven-gate set was run
over, as `-Label p16-frozen`. No code, test, fixture or document it measures moved
after that commit. A reader who needs the tree these readings describe should check
that SHA rather than trust the branch name.

The report was first drafted at `d3e35ad`, and the tree moved twice more under it
while it was being written: `d3e35ad` → `b55a675`, the three dogfood repairs →
`42812fc`, this report and `docs/development/DOGFOOD.md` de-staled. It sits on the
branch `claude/v0.1-autonomous`, against `origin/main`.

Three things bound what the commit identifier means here, and each was measured
rather than assumed:

* **The branch moved twice while this report was being written, and it is still
  moving.** `P16` has **twelve** tasks — `tasks/tasks.json` and
  `progress/state.json` agree on the number, and an earlier draft of this report
  said fourteen, which neither file supports — and lanes landed throughout: `f34e18b`
  (12:03:00) → `b5f44f9`, `P16-T012` (12:10:48) → `d3e35ad`, `P16-T011`
  (12:11:07). The reproducibility command in §2 was run against the first of
  those and again against the second, with identical output. **Two findings
  changed state during writing and the text was corrected rather than left as
  first written**: `d3e35ad` *is* the repair of §5's finding 2 and §8's item 6,
  so what those sections first recorded as an open defect now reads as
  repaired-with-residuals, each residual carrying its own measurement; and the CI
  reading in §6 moved from `in_progress` to `success` for `f34e18b` while this
  was being typed. A reader who needs the tree this report describes should check
  the SHA rather than trust the branch name.
* **The tree was dirty for most of the writing, and the three repairs §8 describes
  were uncommitted for part of it.** `git status --porcelain` returned **15
  modified tracked files and 3 untracked files** at one reading and **8 modified
  and 3 untracked** at the next, as lanes committed what they had finished. The
  three files that carried the repairs — `crates/sure-cli/src/human_report.rs`,
  `crates/sure-cli/src/portable_report.rs` and
  `crates/sure-core/src/pipeline.rs`, with the new
  `crates/sure-cli/tests/verdict_sentence_once.rs` — are **committed at
  `b55a675`**, an ancestor of the commit named above, so the repairs are part of
  the tree these readings describe. An earlier draft of §8 called them "in the
  working tree, uncommitted"; that was true when it was written and is not true of
  this commit, and the section was corrected rather than left standing.
* **`target/` is not part of any of this.** Every reading the repository takes
  under `target/tmp/` is a scratch record — gitignored, reproducible by running
  the command that produced it, and never the evidence a reader should cite. The
  pages this report links to are the tracked record.

## 2. How to reproduce the readings

### The gate set, and the harness that runs it

The repository's definition of "the checks" is seven commands, run in this
order by `scripts/gates.ps1` (read out of that file's `$GateSet` table at
`scripts/gates.ps1:129`, which holds seven rows):

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --no-fail-fast
node scripts/product-evals.mjs
node scripts/validate-bootstrap.mjs
node scripts/taskctl.mjs validate
node scripts/check-non-windows.mjs
```

```powershell
pwsh -NoProfile -File scripts/gates.ps1 -Label <label>
```

`scripts/GATES.md` is the page that says how to invoke it, what environment the
recorded readings were taken under, and what each gate cannot reach. Three of
the seven are cargo subcommands and four are scripts the tree already carried.
The seventh, `node scripts/product-evals.mjs`, reads
`docs/product/PRODUCT_EVALS.md` against `target/tmp/release-gate.json` and exits
1 naming the command that produces the reading rather than printing a number it
did not read; its row sits immediately after the `test` gate because the `test`
gate is what writes its input.

**This report did not run the gate set.** That is the supervisor's harness to
run, and a worker taking a reading from the same tree at the same time is how
two instruments come to disagree later. What this report states about the gates
is what their own tracked definitions say, read at the commit named.

### The environment the readings are taken under

`scripts/GATES.md`, `## The environment the readings were taken under`, records
that the readings in this repository's record were taken under PowerShell 7.6.6
with a **process-scoped** `Bypass` and `LocalMachine RemoteSigned`, and that the
`Bypass` is injected by the tool harness that starts `pwsh` rather than set by
anything in this repository. It matters: six workspace test targets start
Windows PowerShell with a `.ps1`, and a session that does not carry a permissive
process-scoped policy gets `UnauthorizedAccess` from those targets — the shell
refusing to load a file, not the tree. `scripts/gates.ps1` measures that
condition with a probe child rather than inferring it from a policy name, and
refuses with exit 3 when the child cannot load a script.

### What this report measured itself

One command was run in this session, twice, and its output is quoted whole. The
first run was against `f34e18b`, the second against `b5f44f9`; the two outputs
were identical in every line, including the gate contract digest:

```text
$ node scripts/product-evals.mjs
docs/product/PRODUCT_EVALS.md against target/tmp/release-gate.json
gate contract: evaluation/acceptance-manifest.json @ e2f052f10077…
gate decision: permitted

metric                                                            measured     document       verdict
false green rate on mandatory blocker fixtures                    0 of 13      0              ok
fabricated execution claims                                       0 of 1       0              ok
insufficient evidence fixtures that incorrectly become confirmed  0 of 2       0              ok
mandatory repair-regression fixture caught                        1 of 1       100%           ok
secret redaction mandatory fixtures                               unmeasured   (none stated)  ok
benign-mock false-positive corpus tracked explicitly              1 of 1       (none stated)  ok
default user report passes jargon golden tests                    unmeasured   (none stated)  ok

unmeasured on this corpus: secret redaction mandatory fixtures; default user report passes jargon golden tests
  a line for one of these states no number, and is checked to state none.

every one of the 7 metric lines agrees with the gate. The gate is produced by: cargo test -p sure-core --test acceptance_report_runner
EXIT=0
```

Two things in that frame are worth a reader's attention rather than a sentence
of praise. The gate reads **`permitted`** and the corpus contract is pinned to
`evaluation/acceptance-manifest.json` — which holds **20 cases, 13 of them
release-blocking** (counted from the file). And two of the seven metric lines
state **no number at all**, because nothing in this tree computes one; the third
line from the bottom is the eval saying so rather than leaving the reader to
notice an absence.

## 3. Supported stacks

SURE looks for three ecosystems and no others. The vocabulary is
`crates/sure-core/src/discover/mod.rs:86`, and it is closed:

| Ecosystem | What SURE reads |
| --- | --- |
| Node (JavaScript and TypeScript) | `package.json` and the lockfiles beside it |
| Python | `pyproject.toml`, `Pipfile`, `requirements*.txt`, and the lockfiles beside them |
| Rust | `Cargo.toml`, `Cargo.lock`, and the toolchain pin beside them |

**Every project this build is pointed at is reported at support level C,
`inspect_only`.** That is one named constant —
`sure_core::support::CEILING`, at `crates/sure-core/src/support.rs:116` — and it
is the whole of the answer rather than a property of any particular stack:

```rust
pub const CEILING: SupportLevel = SupportLevel::InspectOnly;
```

`docs/product/SUPPORTED_STACKS.md` defines the three levels by what SURE can
**do**: A (`first_class`) is framework-aware discovery *and meaningful
deterministic checks*; B (`generic`) is common manifests and commands *and
approved generic checks*; C (`inspect_only`) is files and configuration, lacking
safe or reliable run semantics. Both A and B include running something, and this
build runs no project code. `support::classify` therefore takes the level as the
weakest of two things — what SURE read, and what SURE can do with it — and a
project whose manifest SURE read perfectly still lands at C, with the report
saying why rather than leaving a reader to find the difference.

`docs/product/SUPPORTED_STACKS.md` records this as **an open decision rather
than a settled one**, and states the other reading — that a level states what
SURE *understands*, that discovery's per-ecosystem `grade` is the whole answer,
and that the ceiling should be `generic`. It is not taken because
`SupportLevel::plain_description` renders `generic` as *"SURE can find how this
project is built and run"*, and SURE cannot run it: a sentence a user would take
to mean checks exist.

**What each stack gets, concretely.** Discovery reads the manifests and derives
the commands the project would use — the `CommandRole` vocabularies in
`crates/sure-core/src/discover/rust.rs:774` and
`crates/sure-core/src/discover/python.rs:1446` — and those commands are
*planned*. In this build every planned command is stopped by the execution mode
and recorded as `NOT CHECKED` rather than passed or failed. A Rust project's
four command findings are `cargo test`, `cargo check --all-targets`,
`cargo clippy --all-targets` and `cargo fmt --check`.

Two stack-specific properties are held in the tree rather than asserted here. A
`setup.py` **is never read**, and the module's own argument
(`crates/sure-core/src/discover/python.rs:25-36`) is the reason: *"There is no
reading it that is not running it, and SURE does not run a project to find out
what it is."* So its presence is recorded as a finding — this project declares
itself in code SURE will not execute — rather than as an absence. A `build.rs`
**is reported and never run** (`crates/sure-core/src/discover/rust.rs:39`), and
`python.rs:100` likewise leaves a `setup.cfg` unread because parsing INI by hand
could mis-parse silently. The full per-ecosystem account is
`docs/architecture/ECOSYSTEM_DISCOVERY.md`.

## 4. Integration tiers

The tier vocabulary is `sure_domain::capability::CapabilityTier`
(`crates/sure-domain/src/capability.rs:13`) and it has three values, weakest
first: **tier 0 `snapshot`** (SURE sees the files on disk and nothing about the
session), **tier 1 `observed`** (SURE sees what the agent did, after the fact),
**tier 2 `protected`** (SURE can decide before an action happens). A report
claims pre-action control if and only if it claims tier 2, and a report that
disagrees with itself is rejected rather than shown to a user.

Five packages ship under `integrations/`:

| Package | What it is | What is verified how |
| --- | --- | --- |
| `claude-code` | plugin package, four hooks, four commands, per-user PowerShell install/uninstall | behaviourally: install-and-remove into scratch and the copy path run by `crates/sure-testkit/tests/integration_thinness.rs`; launchers run against a stand-in binary by `crates/sure-testkit/tests/hook_failure_semantics.rs` |
| `cursor` | plugin package, six hooks, an `mcp.json`, per-user install/uninstall | behaviourally, same two suites |
| `codex` | agent plugin with a `mcp.toml`, four hooks, skills and prompts | manifests and launcher argv asserted; the `.sh` launcher's index mode asserted |
| `copilot` | a **template that nothing loads** | text-verified only — see below |
| `agent-plugin` | MCP bridge, no hook script | four `#[cfg(windows)]` tests run its launcher's failure path |

`docs/integrations/HOOK_FAILURE_SEMANTICS.md:56` records that
`crates/sure-domain/src/capability.rs` already assigns **Tier 1 (Observed) to
Claude Code and Cursor**. Tier 2 is not claimed for anything, and
`docs/security/PROTECTION_MODE.md:55-58` states the consequence: *"The answer is
advisory in this release."* Both integrations are capability tier 1 (Observed)
and their manifests do not confirm that the harness interprets or enforces the
response, so a decision states what SURE *would* do and not what the harness
did.

**One consequence of that is worth stating plainly, because it is the difference
between a protection and a report.** `docs/integrations/HOOK_FAILURE_SEMANTICS.md`
§1.1 quotes the Claude Code hooks page — *"exit code 2 is the only exit code
that blocks through the code alone"* — and records that SURE exits **1** for a
block (`crates/sure-cli/src/report.rs`). On that reading, in a Claude Code
session **SURE's block does not block a tool call**; it is a decision SURE
reports. That is recorded from a vendor page and from this tree's own exit-code
table, and it was **not** measured here: no editor was run.

### What is behaviourally measured, and what is text-verified only

Behaviourally measured, in this tree, by tests that run the thing:

* **Every launcher's failure shape**, by running each launcher this platform can
  run against a stand-in binary that writes markers and exits with a chosen
  status. The table at `HOOK_FAILURE_SEMANTICS.md` §2.2 is that measurement: all
  seven launcher rows relay the status and pass both streams unchanged, and
  **every launcher fails open when SURE cannot be found** — exit 0 with no
  stdout, which the page states plainly is *"indistinguishable, to the harness,
  from SURE deciding 'nothing to report'"*. Three of the four launcher families
  write a note on stderr saying so; `cursor/scripts/sure-hook.sh` **writes
  nothing at all**, so on that one path the only trace a user could find is the
  exit status.
* **The failure inputs at the process level** — empty stdin, non-JSON stdin, an
  unmapped event type, `--source copilot`, no `--source` at all, and a
  `--settings-file` naming a file inside the project. All six end exit 5, a
  failure frame carrying **no `decision` key**, and no store file afterwards —
  `sure history --format json` reporting zero events with `store_present: false`.
  The frame is on stdout under `--format json` and the same two sentences are on
  stderr without it, so a consumer that reads `decision` and stops reads nothing
  where the one word that matters used to be. The first five rows were measured
  2026-09-19 and the sixth 2026-09-21; the sixth is the one a project writes on
  purpose, and it is a decision SURE made rather than an accident it suffered.
* **The per-user install**, including the branch where no symlink privilege
  exists: measured on this machine, which has no Developer Mode, where
  `New-Item -ItemType SymbolicLink` answered `UnauthorizedAccessException` and
  the installer completed with exit 0 on the copy path from a process that was
  not elevated.
* **The reference-resolution order** (`$env:SURE_BIN` → `PATH` → the per-user
  install location), in its first branch only.

Text-verified only, and this is where a reader should slow down:

* **Copilot cannot be verified behaviourally at all, and it also does not work.**
  `sure hook ingest --source copilot` is refused as a source SURE does not know
  (`docs/integrations/HOOK_FAILURE_SEMANTICS.md` §5, whose own narrower grep over
  the normaliser directory and the ingest path returns nothing). So **every**
  Copilot event exits 5 and stores nothing whatever the encoding, and the fix to
  that package is text-verified only. The package's README and its manifest both
  now say it is a template that nothing loads. The reason that matters is that
  Copilot's documented `preToolUse` rule is **fail-closed on a non-zero exit**:
  a user who wired the template up by hand would have every tool call denied,
  with no evidence recorded for any of it.
* **The codex launchers' declared Windows invocation.** `integrations/codex/hooks/hooks.json`
  lines 10, 21, 32 and 43 all read
  `powershell -NoProfile -File "${PLUGIN_ROOT}\\scripts\\sure-hook.ps1"`. That
  form pre-decodes and drains stdin, so `[Console]::InputEncoding` cannot help
  it: non-ASCII payloads stay mangled with `EXIT=0` and `stored: 0`. That is a
  **silent** failure in a shipped package — the same class as the launcher
  defect in §5's findings, one layer out.
* **A real editor loading a plugin, on any platform.** No Claude Code process and
  no Cursor process loaded either package; no plugin was installed into a running
  session and none was watched loading. The install scripts place a package on
  disk and have tests; the load is the harness's step and no harness was run.
  `docs/integrations/INSTALLATION_MATRIX.md` carries this under its own
  `### Cannot confirm` heading, and it is true of macOS and Linux as well as
  Windows.
* **What each harness does with a non-zero, non-2 exit.** That column of the
  failure table is either a quotation from a vendor page (read 2026-09-19, cited
  in the page's §7, pinned by nothing in this repository) or `cannot confirm`.
  Six rows say `cannot confirm`, and each names what would settle it.

## 5. Native Windows status

`docs/development/VALIDATION_WINDOWS.md` is the page to cite, and it says what it
is about in its own third paragraph: the commit it describes is
**`eab162e3af51270a5187083003d81511e48da43d`**, read out of the built archive's
own `RELEASE.txt` rather than remembered.

**The branch has moved far past that commit.** `eab162e` is an ancestor of
`d3e35ad`, and the page itself records that `P16-T002` landed at
`69e1d4692c8e72f789d788bd8b5610987908bea0` while its hostile-path step was still
running, so even the harness's own `HEAD` was one commit past what was built.
Nothing in that record was re-run against a later commit.

### What the run did

On one machine — native Windows 11 `10.0.26200.0`, PowerShell 7.6.6,
`rustc 1.98.1` host `x86_64-pc-windows-msvc`, Node v25.8.1, Git
`2.55.0.windows.3` — every step was run and its exit code recorded:

| Step | Command | Exit |
| --- | --- | --- |
| A environment | `pwsh -NoProfile -File scripts/Test-SureEnvironment.ps1` | `0` |
| B build/package/verify | `pwsh -NoProfile -File scripts/Build-Release.ps1 -OutputDirectory …` | `0` |
| C install | `pwsh -NoProfile -File scripts/Install-Sure.ps1 -Archive … -InstallRoot …` | `0` |
| D doctor | `sure doctor` and `sure doctor --format json` | `0`, twice |
| E check / repair / history | `sure check`, `sure recheck`, `sure history` | `1` / `1` / `0` |
| F Claude Code plugin | `install.ps1` then the launcher driven by hand | `0` |
| G Cursor plugin | `install.ps1` then the launcher driven by hand | `0` |
| H the whole flow again | from `C:\Users\lishi\code\SURE\target\tmp\windows-validation\路径 with spaces 校验\sure repo` | every step passed |

The packaging step verified the archive against its own `.sha256`, extracted it
into a fresh directory, ran the extracted binary from there, and compared the
`running_from` it reported against that directory. `signature not-signed` is a
reading of those bytes by `Get-AuthenticodeSignature`, and the page says what it
is not: a statement about SmartScreen.

Step H is the one worth naming separately: a **checkout at a path carrying both
a space and a non-ASCII character** was cloned, built, gated, packaged,
installed, doctored, checked and wired into both plugins from there, in about
90 seconds of wall clock for the cold build plus 56 seconds for the flow after
it. Nothing failed because of the path, and SURE's own store keyed the project by
the path it was given — with the non-ASCII intact in the file the reading was
read from.

### What it did and did not touch

The record is unusually careful about this and the care is the point:

* Every install target was redirected. `-InstallRoot` pointed at scratch and
  `$env:CLAUDE_PLUGIN_DIR` and `$env:CURSOR_PLUGIN_DIR` pointed at scratch, so
  neither installer could reach `%APPDATA%\Cursor\`, `%APPDATA%\Code\`,
  `%LOCALAPPDATA%\claude-plugins\` or `%LOCALAPPDATA%\agent-plugins\`.
* The user's own per-user directories were measured **before and after**: the
  real store's size, mtime and SHA-256 are unchanged across the whole run, and
  `%APPDATA%\SURE\` did not exist before it and did not exist after. I
  re-measured the store today, read-only: it is still 348 160 bytes with mtime
  `2026-09-18T23:12`, and `%APPDATA%\SURE\` still does not exist.
* The whole flow was driven with `--store-dir` naming scratch, and
  `pwsh -NoProfile -File scripts/gates.ps1` was **not** run.
* `sure` was not on `PATH` before the run and was not put there.

### The five findings that run recorded, and left standing

None was repaired — the page's own argument is that a validation run which
silently fixes what it finds is not a validation run. Two are properties of the
tree and two are the ones a user would actually meet:

1. **The packaging script opens the packager's own store.** `Build-Release.ps1`
   runs the extracted binary as `doctor --format json` with no `--store-dir`,
   deliberately, because a user runs `doctor` against their own store. There is
   no environment variable that could redirect it. What was established is that
   opening it did not change it — on that machine, for that command, which is a
   narrower statement than "safe to run anywhere".
2. **The Windows hook launchers decode their stdin with the console code page.**
   On this machine `[Console]::InputEncoding` is `gb2312`, so a UTF-8 payload
   with a non-ASCII `project_root` is mangled. Measured by A/B on the same
   payload bytes through the installed launcher: **0 sessions, 0 events** with
   the non-ASCII path, 1 session and 1 event with an ASCII one — while the same
   payload sent straight into the binary records the right path. The binary is
   not the problem; the launcher's stdin decode is. `$OutputEncoding` is set to
   UTF-8 further down the same file and protects what is written *to* SURE, not
   what is read *from* the harness. **This one has since been repaired, at
   `d3e35ad` (`P16-T011`)** — which is not the commit this page is a record of,
   and the repair is not a pass: §8 item 6 states the three residuals that
   survive it, each with its own measurement.
3. **An event whose project root does not resolve is dropped without a word.**
   Two routes measured, both silent: a corrupt path (finding 2), and a
   real-looking ASCII path that is not there — exit `0`, the sentence
   `SURE allows this tool request.` on stdout, **0 bytes on stderr and no store
   row at all**. A harness
   whose payload carries no `project_root` gets the process's own directory, and
   if the named store sits inside that tree `Paths::ensure_outside` refuses it,
   so nothing is recorded. Here the *decision* path is not silent — it prints
   the paragraph the reading was quoted from — but the
   *event-only* path, which is what `SessionStart` is, prints nothing and exits
   `0`. `sure history` then answers `"store_present":true,"total":0`: a store
   that exists and is empty looks exactly like one where nothing has happened.
4. **One of the repository's own scripts carries mark-of-the-web.**
   `scripts\Test-SureEnvironment.ps1` has a `Zone.Identifier` stream with
   `ZoneId=3`; its three neighbours have no alternate data streams at all. With
   `LocalMachine RemoteSigned` and no process-scope value, that one file is
   refused from any shell whose process scope is not permissive. **A new clone
   is not affected; this working copy is** — `git clone` does not copy
   alternate data streams. Step A succeeded only because the launching session
   carried a process-scoped `Bypass`, which is inherited.
5. **The two archives built from the same commit do not carry the same
   `sure.exe`.** They differ in **24 bytes out of 9 483 264**, both carrying the
   CodeView `RSDS` signature at the same offset with the 16-byte run that differs
   sitting immediately after it, which is where the linker's per-build debug
   identifier lives. Neither binary contains the string `windows-validation`.
   The page is explicit that this is **not** evidence of a defect and not
   evidence against one.

### What that record does not cover

Named by the page itself as limits rather than left to be assumed: **macOS and
Linux** (nothing in it was taken anywhere but native Windows); **a real editor
loading a plugin**; **MCP** (neither `.mcp.json` nor `mcp.json` was started);
**Windows past `MAX_PATH`** (`LongPathsEnabled` is `0`, `core.longpaths` unset,
deepest path produced 221 characters); **the six gates and `scripts/gates.ps1`**,
deliberately not run; **CI and `release.yml`**, read and not run; **SmartScreen,
antivirus and code signing**; **elevation, services and scheduled tasks**;
**the other three integration packages**; and **a second `sure` on `PATH`** —
the three-way resolver was exercised in its first branch only.

Windows is nonetheless the **primary** platform by `docs/adr/0007-windows-primary-development.md`,
and `CLAUDE.md` makes native Rust MSVC the development target. The Windows
coverage in CI is a `windows-latest` leg of the `rust` matrix plus a
Windows-only bootstrap job (§6); the Windows **artifact** coverage is nothing at
all (see §8).

## 6. Cross-platform status

### CI as it actually ran

`.github/workflows/ci.yml` defines three jobs: `bootstrap-validate-windows`
(`windows-latest`, three steps), `rust` (a matrix over
`[windows-latest, macos-latest, ubuntu-latest]`), and `shellcheck-secondary`
(`ubuntu-latest`). Each `rust` leg runs, in order:
`cargo fmt --all -- --check`, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo test --workspace --all-features --no-fail-fast`, and
`node scripts/product-evals.mjs`. The last
one is in the matrix leg rather than beside the two node gates in the bootstrap
job because its input is the `test` step's own artifact.

Read with `gh`:

| Run | Workflow | Commit | Conclusion | Jobs |
| --- | --- | --- | --- | --- |
| `35559221587` | `ci` | `e8ba638` | **`success`** | all five: `bootstrap-validate-windows`, `rust (macos-latest)`, `rust (ubuntu-latest)`, `rust (windows-latest)`, `shellcheck-secondary` — every one `success` |
| `35559588094` | `ci` | `f34e18b` | **`success`** — all five jobs | `bootstrap-validate-windows`, `rust (macos-latest)`, `rust (ubuntu-latest)`, `rust (windows-latest)`, `shellcheck-secondary` — every one `success` |

**That run finished while this report was being written.** When it was first read
its run-level status was `in_progress` with four of its five jobs complete and
`rust (windows-latest)` still running; by the end of writing it read `completed`
/ `success` with **all five jobs green**, the Windows leg included. So the
reading for `f34e18b` is unambiguous: the suite passed on all three platforms,
and the six `.ps1`-spawning targets §2 and §8 describe passed on Windows in CI
as well as on the developer's machine.

**The commit this report names has no CI run at all.** `gh run list` shows the
run above as the most recent, created `2026-09-21T04:03:20Z`; the two commits
that landed after it — `b5f44f9` at `04:10:48Z` and `d3e35ad` at `04:11:07Z` —
have no run between them. What exists is a green reading for the **second
ancestor** of the commit named in §1, not for that commit itself.

One further CI fact, measured from the run history rather than from a file: the
`ci` workflow was **red for nine consecutive pushes** before `e8ba638`. The last
green `ci` run was on `eab162e` at `02:56:57Z`; the nine failures run from
`a2ba1fd` through `06d3a47`; `e8ba638` is green. That streak is item 1 of the
findings this phase fixed, and its cause was a fixture pinning a sentence a
document had stopped saying.

### `release-dry-run.yml`, and what it produces

Run `35555938432` (`workflow_dispatch`, commit `eab162e`) concluded `success`
with **all six jobs green**, and it is the run that produced the three packaged
artifacts. `docs/development/PLATFORM_COVERAGE.md` is the page that carries that
reading and it does not soften the fourth row:

* `aarch64-apple-darwin`, `x86_64-apple-darwin` and `x86_64-unknown-linux-gnu`
  each build, package, checksum, extract, read the architecture out of the
  extracted binary's own header bytes, and **run the extracted binary from the
  directory it was extracted into**.
* **The two macOS archives are not in one state described twice.** The Intel
  artifact reads `code object is not signed at all`; the arm64 artifact carries
  an **ad-hoc, linker-signed** signature (`Signature=adhoc`) that no certificate
  produced. Neither is a Developer ID, neither is notarized, neither is stapled.
* The Linux archive requires **glibc 2.39 or newer**, read off the packaged
  binary — and the workflow asserts nothing about it, deliberately: the step is
  named *"What this archive asks of its glibc, read off the binary"* and has no
  `exit 1` on any GLIBC condition. **A future runner image that moved the floor
  would not fail any job.**
* **There is no Windows packaging job in it.** Its four `runs-on:` lines are the
  `validate` matrix, `macos-latest`, `macos-26-intel` and `ubuntu-latest`, and
  its only `windows-latest` is a `validate` leg that uploads nothing.
* The three counts differ by platform — 2810 passed on Windows, 2768 on Linux,
  2766 on macOS for the same commit — and that is the expected shape: **a green
  matrix job is three different sets of tests, not one set run three times.**
  `docs/development/GITHUB_WORKFLOW.md` records that the totals are not subsets
  of one another, so a count is not a check that the right tests ran.

### `node scripts/check-non-windows.mjs` — what it can and cannot reach

This is the sixth gate, added because `cargo clippy` with no `--target` on
Windows compiles the Windows `cfg` set, in which `#[cfg(not(windows))]` items are
not used, so a break in the non-Windows arms is invisible on the development
machine. The script's own header records the census that justifies it: **116
platform-conditional sites** in the workspace, measured 2026-09-19.

| Crate | Sites | Reached? |
| --- | --- | --- |
| `sure-domain` | 0 | reached |
| `sure-protocol` | 0 | reached |
| `sure-testkit` | 12 | reached |
| `sure-core` | 100 | **NOT reached** |
| `sure-cli` | 4 | **NOT reached** |

The two it does not reach are the two that need a C compiler for the target:
`sure-core` takes `rusqlite` with `bundled`, so `libsqlite3-sys` compiles
`sqlite3.c` for the target, and it takes `ureq`, which brings `ring` and its own
C; `sure-cli` depends on `sure-core` and inherits it. Both fail in a build
script, so `cargo check` fails there exactly as `cargo clippy` does, and
`--no-deps` does not avoid it. **A crate that cannot be checked is not silently
treated as checked** — the run names it, says the whole-workspace attempt was
made first, and says what would have to be installed to close the gap. The gap
closes by itself on a machine with a cross C compiler: the script attempts the
whole workspace first, and the `NOT CHECKED` section appears only when that
attempt actually failed for that reason.

**What it cannot reach at all, by construction, is the class that only fails when
Unix code runs.** Two of the three defects in the streak's census were assertions
that fail on Unix at run time — a Windows path classified by Unix path rules, and
a Unix branch asserting the wrong `kept_open`. No compile step and no lint on any
platform catches those; only executing the tests on Unix does, which is what CI
is for. **`check-non-windows.mjs` catches the lint or compile break that aborts
the Unix job before its tests run, and nothing more.**

**This report did not run `check-non-windows.mjs`** — it is a gate, and the
figures above are the script's own recorded measurements read out of its header
rather than readings this report took.

## 7. Privacy and execution behavior

### What SURE reads

* The files of the project it was pointed at, by the discovery and detector
  walks, with a declared skip set (`.git`, `node_modules`, `target/`, caches).
* The user's own settings file, when there is one —
  `%APPDATA%\SURE\sure.yaml` on Windows, through the `dirs` crate's
  known-folder conventions rather than a hand-rolled read of `%LOCALAPPDATA%`
  (which silently produces a *relative* path when the variable is unset, and a
  relative path resolves against the directory being checked).
* A project's own `sure.yaml`, which is read but **cannot outrank the user's**:
  the mode in effect is the stricter of the two, so a project cannot loosen what
  the user set.

### What SURE writes

* `%LOCALAPPDATA%\SURE\sure.db` — the record store — written by the commands
  that record: `sure recheck`, `sure repair`, `sure hook ingest`,
  `sure hook allow-once`. **`sure check` creates nothing**, before or after; it
  opens a store only when one is already there.
* `%APPDATA%\SURE\sure.yaml` when the user sets a setting. `privacy.full_recording`
  is off by default and **only the user's own settings file can turn it on** — a
  project naming it is a request its own file cannot grant.
* `--store-dir <DIR>` moves the store, and the location comes from exactly one
  place: the process's own argument vector. Nothing in the project is read for
  it, and there is **deliberately no environment variable** — a variable would be
  the same hole under a different name, since a checked project's harness
  configuration can set the environment of the processes it starts.
* **Nothing is written into the project.** This is asserted rather than
  asserted-here: `crates/sure-cli/tests/cli_contract.rs` runs two hook runs and
  three checks over a project containing a hand-written `.sure` directory and
  then asserts `bytes_under(&project.join(".sure"))` is unchanged, with the
  failure message *"a run wrote into the project's cache directory, which this
  build has no writer for"*, and asserts the machine's own locations untouched.
  The `.sure` project-cache location is declared in `Paths` and has no writer in
  this build.

### What SURE sends

* **Nothing, in this release.** `docs/security/PRIVACY.md` names three modes and
  this release's answer is always the same: *"In this release no check asks for
  model-backed analysis, so the answer is always that no model was consulted"* —
  and a run that cannot support even that says so instead. The report states
  which mode was in effect and whether a model was consulted **on every run that
  gets past reading its settings, including a run that stopped before a
  verdict**, because silence would read both as "nothing left this machine" and
  as "SURE did not look".
* `cloud_enhanced` is **refused rather than accepted**, so no project can claim
  an arrangement SURE does not provide. `fully_local` plus an external provider
  is refused.
* No silent telemetry is required for v0.1.
* The vocabulary carries the capability: `ureq` is a dependency of `sure-core`,
  and nothing in this build calls it on a product path.
* Deletion exists and is a command: `sure history` and `sure history delete`.
  **Nothing is deleted on a schedule**, nothing is deleted without a scope on the
  command line, and a project cannot extend how long its own records are kept.
  The page is also explicit about the limit: a delete removes *sessions*, and a
  session is not the whole store — rows no session event points at are not
  reachable by any command a user can run in this release.

### What SURE runs

**This build executes nothing against a judged project.** That is the single
most important sentence in this section, and it is a named constant rather than a
policy statement: `sure_core::support::CEILING` is `SupportLevel::InspectOnly`
(`crates/sure-core/src/support.rs:116`), and the module header's argument is
checkable rather than remembered — `sure-core` holds **four** `Command::new`
sites, and exactly **one** is on a product path: `crate::fingerprint`'s, which
runs `git`, read-only, to build the content fingerprint. The other three are
`crate::process`'s runner and its `taskkill`, and the browser launcher. None of
the three is on a path from any command a person can run: the runner's one caller
can start only what `crate::enforce` admitted and nothing in the product builds a
`StartSmoke`, the browser launcher has no caller because nothing in the product
constructs a driver, and the chain from a command ends one link short of a
process. Those properties are held by tests (`tests/spawn_sites.rs`,
`tests/browser_probe.rs`) rather than asserted in prose.

`inspect_only` is the default and, per `docs/architecture/EXECUTION_SAFETY.md`,
**the answer whenever anything short of a deliberate arrangement is in effect**.
It is also the only mode this build acts on: the other two modes are decisions
about what *may* run, and a permission changes what SURE may do and not what SURE
does. `docs/security/PROTECTION_MODE.md` says the same about the hook: the answer
is advisory in this release.

Two consequences a user will meet, stated rather than left to be discovered:

* A project whose only planned checks are commands produces
  `not_enough_checked` and exit **1** under the default mode — not exit 3, which
  would mean this build cannot check a project at all. The report says which
  stage did not run and why, and it says that a run with a stage that did not run
  is never reported as clean.
* `sure doctor` searches for the programs a build on this machine uses and
  reports where each one is. That is SURE talking about its own prerequisites
  without running any of them; *probes* would be a claim this build cannot make,
  and it does not make it.

## 8. Known limitations and external blockers

This section is the one that earns the report its keep. Nothing below is
softened, and each item says whether it is a limit this release carries on
purpose or a repair that is owed.

### Repaired in P16, at `b55a675`

**1. The verdict sentence is printed twice.** `crates/sure-cli/src/human_report.rs`
pushes `verdict.aggregate.headline`, then a blank line, then `render_summary`,
whose first line (`crates/sure-core/src/project_verdict.rs:70`) is that same
headline. For the `not_enough_checked` verdict the two are the same frozen
string, so the most-read line of the most-read surface appears twice. Confirmed
by reading the file as the named commit carries it (`git show d3e35ad:…`) and
again as the working tree has it: the two pushes are in both. **Repaired in
`b55a675`** — the headline push is gone from all three human renderers, and
`crates/sure-cli/tests/verdict_sentence_once.rs` now holds the sentence to exactly
one appearance in the terminal, markdown, HTML and JSON forms. Cosmetic, and
squarely in the middle of what a user reads first.

**2. A first `recheck` reports a comparison that never happened.** A virgin store
prints `Against the earlier run: 0 finding(s) still open, 0 closed.` while stage
12 of the same report says `no earlier run left anything open for this project`
and `check.rs`'s own doc comment forbids exactly that duplication. A reader of a
first re-check is told a comparison found nothing open when there was no
comparison. The verdict is not-green either way, so it is a wording defect and
not a false green — but *"0 findings open against the earlier run"* is precisely
the sentence a person would quote. **Repaired in `b55a675`** — `pipeline.rs:1184`
returns `None` when there is nothing to compare, so the sentence is absent *and*
`details.lifecycle` is `null`, and
`a_recheck_with_nothing_to_compare_does_not_report_a_comparison` holds both, with a
control that a fix merely stopping the print would fail.

**3. A documented refusal that does not happen in every state.**
`docs/architecture/CLI.md:425` says, unconditionally, that a store location
inside the project being checked is refused with status 5 before the store is
opened. Measured with absolute paths: absent directory → exit 1 and a full
report; present-but-empty → exit 1; `sure.db` present → exit 5. **The security
property holds in every state** — nothing was read and nothing was written in the
first two, and the third stopped. What does not hold is the documented promise,
which is unconditional, while the code's behaviour is state-dependent. The
decision taken is that the code is right and the sentence is over-broad, because
the same section already says `check` opens a store only when one is already
there. **Repaired in `b55a675`** — the sentence now says "refused when the run
opens the store there", with the measured three-state table, in both
`docs/architecture/CLI.md` and `docs/architecture/STORAGE_AND_DATA_PATHS.md`, and
in the `--store-dir` `--help` text at `crates/sure-cli/src/cli.rs:63`, which made
the same unconditional promise to anyone running `sure check --help`.

### Carried limits, with the measurement rather than the shape

**4. The release gate never compares a number on an unmeasured metric.**
`crates/sure-core/tests/release_gate_runner.rs` builds its gate from the module's
own report, which has no process runner, so `repair-regression` reads
`Unmeasured` there. The match arm that handles it (the `MetricValue::Unmeasured`
arm, `release_gate_runner.rs:283`) asserts only that the claim is *named* in
`unmeasured_metrics` — it never compares the number the document states. Moving
`docs/product/PRODUCT_EVALS.md` from `**100%**` to `**99%**` therefore left that
test at `16 passed; 0 failed`, measured at `P15-T026` and recorded in
`scripts/GATES.md`. The seventh gate closes that hole **for that document**:
`node scripts/product-evals.mjs` exits 1 with `WRONG` on the same edit. **The
tolerance itself is untouched** — a metric this build cannot measure can still
state any number it likes in any other document.

**5. The container-reach measurement counts `#[cfg(test)]` lines inside shipped
files.** `crates/sure-core/tests/adversarial_fixture_detection.rs` documents this
as a property of the measurement at lines 3273–3276: the filter is the file's
path and not the item's attributes, so a test module inside a `src/` file is
counted. **The limit that matters is asserted independently**: the assertion that
no shipped code can reach container execution
(`assert!(reach.execution.is_empty())`, `:3479`) does not read the declared
numbers at all, so no re-declaration can move it.

**6. The Windows hook launchers decoded their input with the console code page —
repaired at `d3e35ad`, with residuals.** Finding 2 of §5, and the difference
between a shipped package that works and one that silently stores nothing for any
project whose path contains a non-ASCII character. `P16-T011` landed the read
repair: `[Console]::InputEncoding=New-Object System.Text.UTF8Encoding($false)` is
now a statement of all four launchers (`claude-code` and `cursor` and `copilot`
at line 6, `codex` at line 4, after the `$ErrorActionPreference` line each opens
with). **Measured by driving the launcher, never by reading it**: under
`chcp 936` with a payload whose root carries `路径 with spaces 校验`, the repaired
launchers store 1 session with the root correct to the codepoint, and the same
payload with an ASCII root still stores its session — so the reading can tell the
two cases apart rather than only seeing a green. **Three residuals remain, each
measured, and the commit does not claim otherwise** — its own words are that
clause 1 is met *"for two of the four packages as declared"*:

* **codex's declared Windows invocation still mangles, and still stores 0 rows.**
  The `powershell -NoProfile -File …` form (which `VALIDATION_WINDOWS.md` also
  records) pre-decodes and drains stdin, so a statement inside the script cannot
  reach it. This is a **carried defect with a measurement, not a pass** — a
  silent failure in a shipped package. Three routes to close it were tried and
  each is recorded with its reason: a read inside the launcher cannot work under
  `-File`, and re-decoding the mangled text is not a repair but a guess dressed
  as one, because nothing in the payload says which of the two readings is the
  true root.
* **Windows PowerShell 5.1 does not apply a script-scope `$OutputEncoding`** when
  it pipes to a native command, so the same class of defect survives on the way
  out for **every** package under that host: with the read repaired and only the
  scope changed, the shipped form hands SURE `??` (`41 3F 3F 42`) and stores 0
  rows, while `$global:OutputEncoding` in a fresh process stores 1 with the root
  correct. Carried rather than fixed, because repairing it would falsify a
  recorded byte-order-mark measurement at `integrations/codex/README.md:287-291`.
* **Which host a real harness runs cannot be confirmed** — no live harness was
  started — so **under 5.1 the read repair is not sufficient for any package**,
  and the lane verified pwsh 7.6.6 and Windows PowerShell 5.1.26100.9444 across
  both invocation styles it can drive rather than generalising from one.

Copilot cannot be behaviourally verified at all, and does not work: see §4. Its
launcher was corrected anyway, because the same two lines are copied across all
four packages and it is the shared shape being fixed. One further constraint is
worth recording because it shaped the repair: the codex launcher is at **39 of a
40-line bound** (`sure-testkit/src/integrations.rs:281`, checked at
`LauncherTooLarge`), so the fix had to stay a single line — and it did, which is
why `integration_thinness.rs:2632` still reads `:5-12`.

**7. `sure check`'s own severity line counts unrun checks, not defects.** The
dogfood run printed `Open findings: 5 Must fix, 5 Should fix first, 5 Can fix later, 3 Note.`,
and 5+5+5+3 = 18, which is the number of checks that **could not
run** — the line below the same report says so. The number is not wrong, and it
errs toward alarm rather than toward a false green, and the machine frame keeps
the two lists apart and names them. It is available to be misread: a script that
parses `Open findings` and stops reads eighteen established defects. A wording
risk on the most-read surface, recorded as such.

**8. The four command anchors say "command at `Cargo.toml`", and none of the four
is declared there.** `grep -n 'cargo test\|clippy\|cargo check\|fmt' Cargo.toml`
returns only `[workspace.lints.clippy]`. The four are derived from the Rust
stack's `CommandRole` vocabulary, with the format role appending `--check`; the
manifest appears in the anchor because it is what makes the stack Rust. A reader
who takes "command at Cargo.toml" to mean "declared in Cargo.toml" looks for a
declaration that is not there. **One word away from being exact.**

**9. `exit::REFUSED` (status 4) is unreachable.** It is defined at
`crates/sure-cli/src/report.rs:100` as *"Reserved… for a refusal that is a
decision rather than a failure"*, carries
`#[allow(dead_code, reason = "reserved by docs/architecture/CLI.md; nothing refuses yet")]`,
and the only other mentions of it in `crates/sure-cli/src/` are a comment
explaining why a call site does **not** use it. A reserved status with no call
site is a status a caller can never receive, so any caller that handles it
handles nothing.

**10. The source manifest is a curated list, and coverage of `crates/` is thin.**
`SHA256SUMS.txt` holds **195 entries**, and the digests are of the **index blob**
rather than of the bytes on disk — `crates/sure-testkit/src/source_manifest.rs`
carries the measurement behind that choice, and the tool never adds a path and
never removes one. **Eleven of the 195 are under `crates/`** (five `Cargo.toml`s
at lines 39–49, four `lib.rs`, one `main.rs`, and one test file), while the tree
tracks **154 `.rs` files under `crates/*/src/`**. So 149 of those 154 files —
including `crates/sure-core/src/support.rs`, where `CEILING` lives — can change
with no digest moving. `docs/development/INSTALL_WINGET.md`,
`crates/sure-cli/tests/quickstart_flow.rs` and `scripts/gates.ps1` are unlisted
too. **The correct statement of this debt is narrower than "no `crates/` file is
in the manifest": the manifest names a skeleton of each crate and not its
sources.** Measured against `git ls-files`, the tree tracks **607 files and the
manifest lists 195, so 412 tracked files move no digest** — 219 under `crates/`,
113 under `fixtures/`, 37 under `integrations/`, 20 under `docs/`, 12 under
`scripts/`, 4 under `schemas/`, 4 under `packaging/`, 1 under `.github/`, plus
`Cargo.lock` and `SHA256SUMS.txt` itself. There are **zero stale rows**: every
listed path is still tracked. So the manifest is not lying; it is incomplete.
**Five of the gate instruments are among the unlisted**: `scripts/gates.ps1`,
`scripts/product-evals.mjs`, `scripts/check-non-windows.mjs`,
`scripts/measure-tests.mjs` and `scripts/GATES.md`; only
`scripts/validate-bootstrap.mjs` and `scripts/taskctl.mjs` are listed. An
instrument can be changed, therefore, and no digest moves — while the manifest's
stated purpose is to say what the tree was. `scripts/GATES.md` records what that
means for tracking the harness itself: adding a file the manifest does not list
changes no digest, so the runner came with no manifest cost.

**11. Smaller residuals, each carried with its own measurement.** Each of these
has a measurement recorded in `progress/HANDOFF.md` at the commit that found it,
and this report did not re-measure them: the *"budget expired before the window
closed"* gap in `sure hook allow-once`; the stale `target/tmp/release/` Windows
archive (I confirmed only that it is still there and still dated 2026-09-19,
4 045 010 bytes, which is the size `docs/development/PLATFORM_COVERAGE.md` also
records); the allow-on-unresolvable-root policy question; a commit whose subject
line misdescribes what it does; the two `sure-domain/src/execution.rs` line
numbers the inventory names for the same concern; the reachability of the
`local_command` analysis provider; and the shell-environment fact behind the
`.ps1`-spawning tests, which `docs/development/WINDOWS.md` now records in full:
**six of this workspace's 66 test targets start `powershell.exe` with a `.ps1`
path**, and from a session carrying no process-scoped execution policy **all six
fail — 29 failures across the six targets**, every one of them Windows
PowerShell refusing to load a file
(`FullyQualifiedErrorId : UnauthorizedAccess`). The variable is not which shell
it is; it is whether the session that starts `cargo test` carries a
**process-scoped** policy, because only a `Process` value is inherited. A
hand-opened PowerShell 7 window reports `RemoteSigned` while its
`powershell.exe` child still refuses a local `.ps1`, so "use PowerShell" is not
the rule and a machine- or user-scoped policy change must not be proposed as
one: it would make the refusal invisible on every machine it was applied to,
including machines where it is hiding a real defect. This is an environment
condition and not a product defect, and `scripts/gates.ps1` enforces the
condition before it runs anything — refusing with exit 3 rather than emitting a
red `test` gate caused by the shell, which is a red this repository has already
once paid for reading as a reading of the tree.

**12. `progress/HANDOFF.md` is 24 720 lines.** It is the record of the phase and
it is not a document a reader can read end to end. That is a real cost of the
way this repository keeps evidence: the readings are all there, and finding any
one of them means knowing what to search for.

**13. The suppression census does not read the shipped integration launchers,
and every one of them opens with the token it counts.** `scripts/gates.ps1`'s
`$CensusRelative` array (lines 191–198) names six files — `scripts/gates.ps1`,
`scripts/validate-bootstrap.mjs`, `scripts/taskctl.mjs`,
`scripts/check-non-windows.mjs`, `scripts/measure-tests.mjs` and
`scripts/product-evals.mjs`. All six are the harness. The tree, meanwhile,
contains **five files under `integrations/` that open with
`$ErrorActionPreference='SilentlyContinue'`**: all four
`integrations/*/scripts/sure-hook.ps1` at line 1, and
`integrations/claude-code/scripts/sure-mcp.ps1:7`. So the census reports a count
of zero **about a tree that contains five**, and the census line is quoted in
every reading this repository records. Both sides have to be stated or the
finding is unfair. **The token is defensible where it sits**: a hook that throws
can break the agent session it was installed into, and
`Get-Command sure -EA SilentlyContinue` is a probe whose expected answer is "not
found". What is not defensible is that nothing counts it there. T011 then
measured the consequence: a codex payload with a non-ASCII `project_root` under
code page 936 returns **exit 0, `stored: 0`, and nothing on stderr** — and a
silent failure in a shipped package is exactly what a suppression token
predicts. This is a **finding, not a repair**: widening the census to
`integrations/` would redden on the five above, and changing what they do with
errors is a decision about the launchers' error policy rather than about the
census.

### External blockers

Nothing in this list can be closed by work inside this repository, and each was
read rather than assumed:

* **`release.yml` is not on the default branch, so it cannot be dispatched at
  all.** `git ls-tree origin/main --name-only .github/workflows/` returns exactly
  two paths, `ci.yml` and `release-dry-run.yml`, and `gh workflow list --all`
  reports `ci`, `release-dry-run` and `Dependabot Updates` — **`release` is not
  among them.** So the four-archive draft release, the nine named assets, the
  download-rehash round trip and the Windows ZIP on a runner are all statements
  about what a file says, and the file is not in a place where anything can act
  on it. This is a **stronger** statement than "it has never run".
* **No Windows artifact has ever been built on a runner.** The only Windows ZIP
  that exists was built by hand on a developer's machine, from an earlier commit
  (`ae3671…`), and is stale by construction. The only artifact this project ships
  for its **primary** platform is the one artifact no run in this repository has
  ever produced.
* **Zero tags and no GitHub Releases.** `git tag` lists nothing and
  `gh api repos/lichman0405/SURE/releases --jq length` answers `0`. `release.yml`'s
  one required input is a tag that must already exist.
* **Nothing is in a package manager.** No WinGet package, no Visual Studio
  Marketplace extension, no Open VSX extension, no npm package. A WinGet manifest
  *template* exists and is rendered by a script; submitting it is a pull request
  against a repository this project does not own.
* **macOS signing: no Developer ID, no notarization, no staple.** A runner
  executing a binary out of its own scratch directory is **not a launch through
  Gatekeeper**, so what a user actually meets on first launch is unobserved here.
* **The Linux floor is a reading, not a threshold.** glibc 2.39 or newer, which
  does not run on Ubuntu 22.04 (2.35) or Debian 12 (2.36). GitHub's own
  annotation on run `35555938432` says the `ubuntu-latest` label will migrate to
  Ubuntu 26 beginning 2026-10-19, which would change the builder's libc; because
  the floor is a reading, **no job would fail when it happens**.
* **SmartScreen is not measurable here.** It is a Microsoft service driven by
  telemetry this project does not hold. `not-signed` is a reading of the bytes,
  not a statement about whether a machine warns.
* **A plugin installing is not an editor loading it, on any platform**, and no
  run in this repository starts an editor.
* **A green CI job on a hosted runner is not a statement about a user's
  machine.** Every CI reading was taken on a GitHub-hosted ephemeral image, and
  none of them is the reader's machine.

## 9. The dogfood reading

`sure check` was pointed at SURE's own repository. `docs/development/DOGFOOD.md`
is the record (`P16-T007`), and its own framing is the one to keep:
**SURE's own repository is the hardest project SURE will ever be pointed at**,
because it is the one project that contains both SURE's detection patterns as
source text and SURE's deliberate false-completion fixtures.

The run was made with `target/debug/sure.exe` under the documented default mode,
`inspect_only` — no flag was needed, and no settings file exists on the machine
so nothing could have moved the mode. The store was pointed at a scratch
directory outside the project with `--store-dir`, and the real store is
byte-identical before and after (the record's digest is the one this report
re-measured today, from its size and mtime: unchanged).

**What it said, in the record's own words:**

```text
Could not check
---------------
SURE checked 0 of 18 checks. 4 checks were skipped and 14 checks could not run.
```

```text
Open findings: 5 Must fix, 5 Should fix first, 5 Can fix later, 3 Note.
18 check(s) could not run or were skipped. 1 of them are critical. (run the tests)
```

Exit **1**, `outcome: "not_green"`, every one of the eighteen entries at
`Status: Cannot confirm`, and stage 5's own explanation: the fourteen static
detectors have **no runner in this build**, so none reported a result and each is
recorded as unknown rather than passed; the four cargo checks were stopped by the
execution mode. The honest headline is **not** "SURE found five must-fix problems
in its own repository". It is: SURE planned eighteen checks, ran none of them,
located fourteen places worth looking at, and refused to call the project clean.

**Of the fourteen anchored candidates, twelve are false as defects and two are
true and expected. Zero are material.** The record judges each one by reading the
line it names, and every anchor is literally true — the file really does contain
that text on that line. The twelve are false for three separable reasons, and
they are not one bug:

* **Eight are SURE reading its own pattern library as if it were a project.**
  `demo_data_heuristics.rs:239` is `lower.contains("lorem ipsum")`: the line
  flagged as "hard-coded demo or chart values" is the line that *defines* what a
  hard-coded demo value is. There is no rule that excludes the detector modules
  from the scan, and the run's own framing — *"A candidate is something to look
  at, not a defect SURE has established"* — is what keeps that honest.
* **Three are in-file Rust test modules read as production code.** The candidate
  context filter classifies a **path** and nothing else, and returns `Product`
  when uncertain — the conservative direction, and the one that costs, since a
  test fixture inside a `src/` file is graded with production gravity.
* **One is a line-level pattern match that is simply too wide** — a guard clause
  matched as if it were a function that always succeeds.

**None of these is a false green.** A false green would be *stating* a defect
that is not there; this run states the opposite, and the verdict is "not enough
could be checked". What the fourteen produce is **noise at a severity label** —
an entry reading `Severity: Must fix` over a line that is a doc comment.

**The control run, and the finding that matters most for reading all of this.**
The same command was run against a scratch two-file Rust crate with deliberately
planted patterns — a `return Ok(());` no-op, a `tok_visa` literal, a
`.status(200)` literal, a `demoData` array, a `lorem ipsum` literal, a `TODO`
comment and a JS click handler that does nothing. Every planted pattern was found
at exactly the right line. And the verdict was **the same verdict shape**:
`not enough could be checked`, exit 1, `checked 0 of 10`.

Two things follow, and a reader must not skip the second:

* **The candidate scanner localises accurately.** The false positives on SURE's
  own tree are not the scanner failing to find things; they are the scanner
  finding exactly what it was built to find, in a repository that contains the
  vocabulary of the thing it looks for.
* **This build's verdict does not distinguish the two projects.** A repository
  holding SURE's deliberate adversarial fixtures and a scratch crate full of
  planted false-completion patterns both come back "not enough could be checked",
  because no check runs against either. **A reader must not take the shared
  verdict as evidence that SURE's repository is either sound or unsound.** The
  run establishes neither. The *difference* between the two lives entirely in the
  candidate list.

The dogfood lane also turned up three defects no test did — the doubled verdict
sentence, the first-`recheck` comparison line, and the state-dependent store
refusal. All three are §8 items 1–3, open at the commit this report names. That
is the shape of a dogfood result that paid for itself: the tests were green
throughout.

## 10. What this report did not measure

Named rather than left to be assumed, because a report is a reading and this one
has edges:

* **The gate set.** `scripts/gates.ps1` was not run, and no `cargo` command was
  run. Every statement about the gates is read out of their own tracked
  definitions, and every test count quoted from them is attributed to the page
  that recorded it.
* **`node scripts/check-non-windows.mjs`.** Not run. The census figures in §6 are
  the script's own recorded measurement of 2026-09-19.
* **The `ci` run on the commit this report names.** `d3e35ad` has none, and
  neither does its parent. The most recent `ci` run is on `f34e18b`, two commits
  back, and that one is green on all five jobs — a reading taken after it
  completed mid-writing, not a prediction. No run exists for the commit named in
  §1 itself.
* **`sure check` against this repository.** Not re-run here. §9 is
  `docs/development/DOGFOOD.md`'s record of a run made with a debug binary built
  before `HEAD` moved, and the record says so itself. What a reader should expect
  to match exactly is the **shape** of the answer, not the fingerprints or the
  anchor line numbers — the tree moved during and after that run.
* **Anything built from this tree.** No release archive was built, installed or
  run. §5's archive readings are `VALIDATION_WINDOWS.md`'s and are about
  `eab162e`.
* **A real harness.** No editor, no Copilot surface, no Codex run and no MCP
  server was started. §4's "what each harness does" column is citations, not
  measurements.
* **The state of `origin/main`.** Read only through `git ls-tree` and
  `gh workflow list`, for the two facts in §8.
* **The `%LOCALAPPDATA%\SURE\sure.db` store's contents.** Its size, mtime and
  digest were read; it was never opened, and nothing in this report was written
  to it.

The claim this repository is built to refuse is a confident sentence nobody
measured. Where that was the only kind of sentence available, this report has
written down what would settle it instead.

One last disclosure about this document itself: **it was checked structurally
rather than by a Markdown linter, because no Markdown parser is installed in
this tree** — there is no `node_modules`, and this report did not install one.
What was actually checked is stated in the hand-back that accompanies it:
balanced code fences, well-formed headings, consistent table columns, no inline
code span split across a line break, and a gapless numbered sequence through
§8. That is a check on shape, not on prose, and a reader who wants a lint of the
prose will have to connect one.

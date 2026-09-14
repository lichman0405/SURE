# Ecosystem discovery

Ecosystem discovery is the second half of step 1 of
`docs/architecture/CHECK_PIPELINE.md`. The first half — which files SURE will
look at, and which it will not — is `docs/architecture/PROJECT_DISCOVERY.md` and
is implemented by `sure_core::scan`. This half is **reading a few named files and
saying what they claim the project is**; it is implemented by
`sure_core::discover` (`crates/sure-core/src/discover/`).

The whole of this document is one requirement:

> **A manifest states a request, not a fact.**

`"test": "jest"` in a `package.json` means the project *declares* that command.
It does not mean `jest` is installed, that the script runs, or that a single test
passes. `"react": "^18"` is a range handed to a resolver, not the version on
disk. SURE has run no resolver and no package manager, so it has nothing to say
about what either resolves to, and everything here is written so that it cannot
say it by accident.

## The distinction the reading is built on: not there vs. not readable

Every caller asks the same first question — *is there a `package.json` here?* —
and there are four different answers:

1. there is nothing at that name;
2. there is something, and it is a link, a directory or a pipe, which SURE does
   not read;
3. there is a file, and SURE could not turn it into a value;
4. there is a file, and here is what it says.

**Collapsing (1) and (3) into an `Option` is the mistake this half is built to
make impossible**, and it is not a corner case. A project whose `package.json`
failed to parse would be reported as a project with *no manifest*: SURE would
say the project declares no scripts, no dependencies and no package manager, and
every one of those would be a claim about a file it never read. That is a false
statement about the project wearing the clothes of a negative finding, which is
the failure `CLAUDE.md` calls more serious than a visible error.

So no type in `discover` answers that question with an `Option`:

- `read::ReadFile` — what reading one named file produced, one arm per answer
  above, with no value accessor that lets the other three be skipped.
- `node::ManifestState` — `Read(Box<Package>)`, `Absent`, `Unread(UnreadReason)`.
- `node::MemberManifest` — `Present`, `Absent`, `NotReadable(kind)` for a
  workspace member, answered without reading it.

`UnreadReason` carries the seven ways a file that is there can fail to become a
value: `OutOfBudget`, `TooLarge`, `Unreadable`, `NotText`, `NotParsed`,
`WrongShape`, `NotReadableKind`. Each has a constant sentence
(`plain_description`), a stable name (`as_str`), and — for the three that carry
one — a `detail()` that is the operating system's or the parser's own words,
kept as data and never spliced into a sentence.

**An unread file is a finding about the project, not a diagnostic.** This is why
`Discovery::unread` is a list in the result rather than a line in
`crate::diagnostics`: that module is what SURE says about *its own* work and is
documented as never evidence about the project. A project with an unreadable
`pnpm-workspace.yaml` and a readable `package.json` is a project SURE can still
say something true about; what it must not do is report the readable half and
stay silent about the half it lost.

### The rule, enforced structurally rather than by convention

`read_manifest` in `node.rs` is **one function**, not a reader plus a converter.
The first version of the file had them apart, and it was wrong in exactly the
way this section describes: `[1, 2, 3]` parses as JSON, so the reader had
nothing to record, and the shape failure the converter produced reached
`ManifestState::Unread` without ever reaching `Discovery::unread`. The same
project was described two ways by two fields of one result. Pairing them means
every arm that produces an `Unread` sits in the same `match` as the line that
records it, so there is no ordering or convention for a later edit to get wrong.

## What SURE looks at, and what counts as a project

| File | What it answers |
| --- | --- |
| `package.json` | the whole of what the project declares |
| `package-lock.json`, `npm-shrinkwrap.json`, `yarn.lock`, `pnpm-lock.yaml`, `bun.lockb`, `bun.lock` | which package manager the project used |
| `pnpm-workspace.yaml` | workspace members, for the one manager that keeps them outside `package.json` |
| `tsconfig.json`, `jsconfig.json` | that this is a TypeScript project, when the manifest does not say |

**Four things make this a Node project, and every one is a project-level file:**
a manifest, a lockfile, a pnpm workspace file, a compiler configuration. Source
files are *not* on the list. A directory containing a `.js` file is a directory
containing a `.js` file, and calling that a Node project on the strength of one
stray file is the kind of inference this product exists not to make.

A lockfile is **not parsed**. Its contents are a resolved dependency graph SURE
has no use for and that costs megabytes to read; what is wanted from it is one
bit — *this project has a `pnpm-lock.yaml`* — and that bit is the file's
existence. Which is also why the manifest byte budget is never spent on one.

Names are matched the way the platform matches them. On Windows a project with
`Package.json` is a project with a `package.json`, because Windows would open it
and a lookup that missed it would report the project as having no manifest. The
rule comes from `ScanOptions::case` rather than from the module, so both
platform rules are testable on one machine.

`Tree` is built once from the `Scan` the walk already produced, rather than
asking the filesystem again — so discovery cannot disagree with the scan it
reports beside it.

## The package manager, and why it is not "the one with a lockfile"

Projects get this wrong, and SURE reporting a confident answer where the project
contradicts itself is worse than SURE reporting the contradiction. Three kinds of
evidence are collected and kept apart, strongest first:

| Evidence | Where it comes from | What it means |
| --- | --- | --- |
| `Declared` | the `packageManager` field | the project's own statement about itself; Corepack reads it |
| `Lockfile` | a lockfile exists for that manager | that manager has run here |
| `EnginesRange` | an `engines` range names it | a hint about what a contributor should install, and nothing more |

`Managers::disagreement()` reports the two cases where the evidence points two
ways — a declaration naming one manager while the lockfile names another, or two
lockfiles for two managers — and `Managers::agreed()` answers `None` for both
rather than picking a winner. **There is no code path here that silently prefers
one lockfile to another.** A `packageManager` field naming something not in
`PackageManager::ALL` is not guessed at; the field is carried verbatim and no
manager is named from it.

## Workspaces

Both spellings of the field are read — `"workspaces": ["packages/*"]` and
`"workspaces": { "packages": ["packages/*"] }` — and `pnpm-workspace.yaml` is
read as a third source. Patterns are carried verbatim and resolved against the
same `Tree`.

- **The root is never its own member.** The seen-set is seeded with the root's
  own lookup key, so `"."` in a pattern list resolves to the root and is then
  dropped as already seen.
- **A pattern cannot leave the project.** `contained_relative` accepts only
  `Component::Normal`, so an absolute path, a root, `.` and `..` are refused as
  `NotInsideProject`. The patterns are project-controlled text, and this is the
  same containment rule the fingerprint applies to the paths Git reports.
  **"Absolute" is the platform's idea, not a spelling**: `C:\Windows` is refused
  on Windows and is one relative name on Unix, where a backslash is an ordinary
  character and a file may legally be called that. Accepting it there is correct
  — the name is inside the project and resolves to nothing — and a test asserting
  the Windows answer on every platform **failed on the macOS and Ubuntu jobs**
  before this sentence was here.
- **`**` is refused, not approximated.** A pattern containing it is
  `UnsupportedPattern` — a statement about the pattern — which is kept apart
  from `NoMatch`, a statement about the project. Answering one with the other
  would turn "SURE cannot read this pattern" into "this workspace is empty".
- **A member whose manifest is absent is still a member.** `MemberManifest`
  answers `Absent` from the walk without reading anything, so a workspace whose
  members are declared but not materialised is described as that.
- **Truncation is recorded.** At `max_workspace_members` the list stops and
  `Workspaces::truncated` is `true`. A workspace with ten thousand members is a
  project SURE cannot describe member by member, and saying so is the useful
  answer.

## Scripts

`Package::scripts` is verbatim: the name the project wrote and the command it
wrote, neither interpreted. An entry whose value is not a string is not dropped
— it is recorded in `scripts_not_commands`, because "the project declared a
script and SURE cannot read it as a command" and "the project declared no such
script" are different facts.

`ScriptRole` is **SURE's** list, not the project's: `build`, `test`, `lint`,
`typecheck`, `format`, `dev`, `start`, `clean`. That is the point — a project
with no `test` script is a finding, and it can only be reported by looking for a
name the project did not choose. `conventional_scripts()` returns **a row for
every role whether or not it is declared**, because silence is not a finding.
One name per role: a project that spelled its test script `test:unit` has not
declared a `test` script, and mapping one onto the other would be SURE inventing
a convention and then running a command on the strength of it.

`command_for(manager, role)` returns the command a person would type — `npm test`
and `npm start` without `run`, `npm run build` with it, `{manager} run {name}`
for every other manager. It is **returned and never executed**: choosing a
command is a plan, and running it is a later step with its own authorisation.

## Frameworks and tooling

`TOOLS` is a fixed table of package names and the role each one has —
meta-framework, web framework, server framework, test runner, end-to-end test,
linter, formatter, bundler, monorepo tool, runtime, language. A `Tooling` value
stores the **table's own `&'static str`**, never the project's spelling, so no
dependency name a project wrote can reach a sentence SURE prints. A package not
in the table is not reported at all, which is a deliberate silence: "SURE did not
recognise this package" is not a finding, and guessing from a name would be. One
package may appear twice when it is genuinely two things — Biome lints *and*
formats, and both are reported, because picking one would be SURE choosing which
half of a tool to mention.

A dependency's `requirement` is stored exactly as the manifest wrote it, and
dependencies are grouped by the section they were declared in —
`dependencies`, `devDependencies`, `peerDependencies`, `optionalDependencies`.
`bundledDependencies` is deliberately **not** a section: it is an array of names
with no versions, every one of which must also appear in `dependencies`, so
reading it would add nothing and reading it as a fifth section would report each
bundled package twice.

## TypeScript

Three pieces of evidence, each with its own `Source`: a compiler in the
dependencies, a `tsconfig.json` or `jsconfig.json`, and `.ts`/`.tsx`/`.mts`/`.cts`
source files. Counted files are evidence, and `source_files` is a count rather
than a list — a project with three thousand `.ts` files does not need three
thousand paths in a discovery result.

## Support levels

`grade` returns the level and the sentence together, as constants, so a level and
its reason cannot be assigned in two places and disagree.

| What was found | Level | Why |
| --- | --- | --- |
| a readable `package.json` | `Generic` | SURE can find how the project is built and run |
| a `package.json` it could not read | `InspectOnly` | it can only look at the project's files |
| no manifest, but a lockfile | `InspectOnly` | it is plainly a Node project and nothing is declared |
| no manifest and no lockfile | `InspectOnly` | only `tsconfig.json`/`pnpm-workspace.yaml` said so |

## What is bounded, and how

Limits are on work, in the same spirit as `ScanOptions` and `FingerprintOptions`,
and every one that is reached is reported rather than applied quietly.

| Limit | Default | Reaching it |
| --- | --- | --- |
| `max_manifest_bytes` | 8 MiB | `TooLarge` — a refusal, **not** a parse of the first however-many bytes |
| `max_manifests` | 512 | `OutOfBudget` for the manifests after it |
| `max_workspace_members` | 512 | `Workspaces::truncated` |
| `scan.max_depth`, `scan.max_entries` | see `PROJECT_DISCOVERY.md` | as that document says |

Two rules are taken from the rest of SURE rather than decided here. **A link is
never read through**: `crate::scan` does not follow one and `crate::fingerprint`
records one by its target, so a manifest that read through a link would be the
one place in the product that reads whatever a project points at, including
somewhere outside the project. A manifest at a link is `NotReadableKind`. And
**reaching a byte limit is a refusal**: half a `package.json` is not a smaller
`package.json`, it is a document that fails to parse with a message about a
syntax error the project does not have. The limit is checked after reading one
byte more than allowed, so a file that grows while SURE reads it is caught too.

An absent optional file does **not** spend the manifest budget: `Probe::Nothing`
and `Probe::Other` are answered before `Budget::take()` is called, so a project
with no `pnpm-workspace.yaml` does not pay for looking.

## What discovery does not do

**It runs nothing.** No `npm`, no `node`, no resolver, no network. Reading a
manifest is not executing a project, and the whole of step 1 is on the
inspect-only side of `docs/architecture/EXECUTION_SAFETY.md`. A test writes a
`build` script whose command would leave a file behind and requires that the file
does not exist afterwards.

**It does not look at `node_modules`**, and not only because the scan leaves it
out. What is installed is `node_modules`'s answer, and a directory that is
usually absent, usually stale and never committed is not a fact about the project
a person can act on.

**It does not decide whether the project is good.** It says what the project
declares and at what support level; whether the declaration is honest is what the
checks after it are for.

**It is not the diagnostics module.** See above.

## Known coverage gaps

Recorded so they are not forgotten. None is resolved by a task yet.

1. **The byte budget has only been exercised where the manifest is the thing
   that crossed it.** Reaching `TooLarge` for a file that grows *during* the
   read needs a writer racing the reader, which no fixture builds. The `+ 1`
   read is what makes the case correct and nothing on this machine demonstrates
   it firing.

2. **`Unreadable` is not tested against a genuinely unreadable file.** The arm
   is real and is reached in tests by a path that does not exist and by a path
   with an interior NUL byte, refused before any system call. What those do not
   establish is that the operating system produces that error for a file with
   permissions that forbid reading — making one needs a privileged user or an
   ACL change, and `chmod 000` does nothing when the tests run as root. This is
   the same gap `PROJECT_DISCOVERY.md` records for its own `Err` arm.

3. **Case-sensitive lookup is not exercised on this machine for the manifests
   themselves.** `Tree` takes its case rule from `ScanOptions::case`, so both
   rules are testable here, and `lookup_key` is tested directly. What is not
   tested on Windows is a project that has *both* `package.json` and
   `Package.json` — constructible only on a case-sensitive filesystem, so the
   CI Linux and macOS jobs are where it would run.

4. **A workspace pattern's `*` is expanded one level, and only one.** `packages/*`
   is a child directory lookup; `packages/*/sub` works because each component is
   expanded in turn. A pattern relying on brace expansion or character classes —
   which npm itself does not support either — is reported as
   `UnsupportedPattern` rather than approximated.

5. **The link at a manifest's name is a directory link.** `mklink /J` needs no
   privilege on Windows and a file symbolic link does; both Developer Mode and
   administrator rights were probed on this host and are absent. The walk's
   symlink arm runs before it looks at what is at the other end, so the refusal
   is one code path for both kinds — but that is an argument, and what a test
   would show is a link whose target is a *file*. The macOS and Linux CI jobs are
   where it could run.

6. **A file can be *replaced by a link* between the walk and the read.** The walk
   records what is at a name; `read_text` then calls `File::open` on the path the
   walk recorded, and an open follows whatever is at that path *now*. A project
   that swaps `package.json` for a link pointing outside the project, in the
   window between the two, is read through.

   **What bounds it.** Everywhere else, containment is by construction rather
   than by a check that could be wrong: a path a manifest names is resolved
   through the walk's own records (`Tree::is_directory`, `child_directories`)
   and never by joining onto the filesystem, and `read_json` takes the `Probe`
   the walk produced rather than a path, so that what is read and what was found
   cannot come from two different walks. This gap is the one place where the
   filesystem is consulted a second time.

   **What it is not.** It is an **integrity** problem, not a disclosure: the
   content lands in SURE's own report, which is read by the person who could
   already open the file the link points at. It also needs an adversary running
   code on the machine during the same milliseconds as SURE, and it changes
   nothing that is executed.

   **Why it is not closed.** Closing it needs an open that does not follow a
   link — `O_NOFOLLOW` on Unix, `FILE_FLAG_OPEN_REPARSE_POINT` on Windows — and
   the Windows flag is not reachable from `std` without `unsafe`, which this
   workspace forbids. `FINGERPRINTING.md` gap 5 records the same window for the
   fingerprint's reader, where the consequence is a stale digest rather than a
   foreign reading; the two are the same shape and neither is closed. Recorded
   here rather than left implicit because a **push security review flagged this
   file for "path traversal / symlink TOCTOU"** on 2026-09-14 — see
   `progress/HANDOFF.md`, which also records that the notification carried no
   finding text and that this gap is what an inspection of the file found.

## Enforced by

| Statement | Where the meaning lives | Enforced by |
| --- | --- | --- |
| A finding always names the file it came from | this document | every `Source` field; `the_conclusion_names_the_files_it_rests_on` |
| "Not there" is never "there and unreadable" | this document, `discover/read.rs` | `a_manifest_that_is_valid_json_and_not_a_manifest_is_unread_too`, `a_manifest_that_is_not_json_is_unread_and_never_absent`, `every_reason_says_which_of_the_four_answers_it_is` |
| An unread file is recorded in the result, not only in the state | this document, `discover/node.rs` `read_manifest` | the same two tests' assertions on `Discovery::unread` |
| A link is never read through | this document, `PROJECT_DISCOVERY.md` | `a_link_named_package_json_is_not_read_through` (a real link out of the project, and a real manifest at the far end) |
| A byte limit is a refusal, not a partial read | this document | `a_file_too_large_to_read_is_unread_rather_than_half_read` |
| The manifest budget is finite and reported when spent | this document | `a_manifest_sure_ran_out_of_budget_for_is_unread_and_never_absent`, `a_budget_of_zero_reads_nothing_and_says_so_rather_than_reading_anything` |
| A pattern cannot leave the project | this document | `a_workspace_pattern_cannot_reach_outside_the_project` |
| The root is never its own member | this document | `the_root_is_never_a_member_of_its_own_workspace` |
| Two pieces of manager evidence pointing apart is reported | this document | `a_project_that_names_one_manager_and_locks_another_is_reported_not_resolved`, `two_lockfiles_for_two_managers_are_a_disagreement`, `two_lockfiles_for_two_managers_are_a_disagreement_and_two_for_one_are_not` |
| No project string reaches a sentence | this document | the fixed `TOOLS` table; `Source::says` is `&'static str`; `a_tool_is_named_from_the_table_and_never_from_the_manifest` |
| Discovery runs no program | this document, `EXECUTION_SAFETY.md` | `discovery_runs_none_of_the_scripts_it_reads` (writes a marker, requires it absent) |
| A conventional role with no script is still a row | this document | `every_conventional_role_has_a_row_whether_or_not_it_is_declared` |
| Order is fixed and independent of the filesystem | `PROJECT_DISCOVERY.md` | `discovery_is_a_function_of_the_project_and_not_of_the_run` |

The "runs nothing" test is worth a note: no run of the code demonstrates that a
program was never executed, so the test also greps `discover/{mod,node,read}.rs`
for `process::Command` and `std::process`. That catches the apparatus and not
the guarantee, and says so.

## Related decisions

| ADR | Topic |
| --- | --- |
| 0001 | Rust local core and crate boundaries — `discover` is in `sure-core`, and it reuses `scan` rather than walking again |
| 0002 | Local-first privacy — reading a manifest tells nobody anything and sends nothing anywhere |
| 0007 | Windows primary development — case-insensitive lookup and spaces in paths are primary cases |
| 0010 | Frozen domain semantics live in code — `StackClassification` and `SupportLevel` are the domain's types, re-exported rather than restated |
| 0012 | Diagnostics are records, not a logging framework — and an unread manifest is not a diagnostic at all |

[`Discovery`]: ../../crates/sure-core/src/discover/mod.rs
[`UnreadReason`]: ../../crates/sure-core/src/discover/read.rs
[`NodeProject`]: ../../crates/sure-core/src/discover/node.rs

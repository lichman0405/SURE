# Project discovery

Discovery is step 1 of `docs/architecture/CHECK_PIPELINE.md`: deciding which
files in a project SURE will look at. It is implemented by `sure_core::scan`
(`crates/sure-core/src/scan/`).

Discovery is small enough to be mistaken for plumbing, and it is the worst place
in the pipeline for a mistake. Everything after it reads the result — the
fingerprint, the stack detection, the readers, the checks, the report — and
nothing after it can tell that a file was missing, because a file that was never
listed looks exactly like a file that is not there. A loss here becomes a verdict
that says "nothing was wrong" about something nobody looked at.

So the whole of this document is one requirement: **a scan that did not look at
something must be able to say so.**

## The three guarantees

### 1. It stays inside the project it was given

Containment is by **construction**, and the construction is two rules:

1. A child's path is its parent's path joined with exactly one name taken from
   `DirEntry::file_name()`. That name comes from the operating system as a single
   component: it cannot contain a separator and cannot be `..`.
2. Links are never followed below the root.

There is therefore no reachable state in which the walk holds a path outside the
root, which is why [`SkipReason`] has no `Outside` variant: a variant for an
unreachable state is a claim the code cannot make, and a caller would have to
write an arm for something that never happens.

This is the same rule as the one P1-T011 applied to consent sources, in a
different place: **do not offer a name for something that cannot be produced.** A
`ConsentGrantor` there must not name organization policy, because a caller could
then name a source it can never obtain, and that is how a documented feature
becomes a believed one. Here a `SkipReason` must not name a way of leaving the
project, because a reason a caller can handle is a reason a caller believes can
occur — and the next step after believing it can occur is writing the handling
code, and the step after that is concluding the construction-based containment is
redundant because the case is reported anyway.

Containment does not need reporting, because it cannot fail. If a future change
makes leaving the project possible, the correct response is to add the variant
**and** a test that reaches it, in one change — not to add the variant quietly so
that a later reader infers the property from the enum.

Links are a loss anyway — see [`SkipReason::NotFollowed`] below. "Never followed"
and "never leaves the project" are two separate statements and both are needed:
the second would still hold if a link were followed, and the first is what stops
a scan of `C:\work\app` from reading `C:\Users` because somebody put a directory
junction in the tree.

A project *reached through* a link is the exception, and only at the root. If the
caller names `C:\work\current`, which is a junction to a checkout elsewhere, that
junction is the project the caller asked about and `fs::metadata` follows it. The
distinction is authorship: the root is the caller's decision, everything below it
is the tree's.

### 2. It is bounded

Two limits, both in [`ScanOptions`], both on *work done* rather than on project
size:

| Limit | Default | Exceeding it |
| --- | --- | --- |
| `max_depth` | 32 | The level is not entered; one `TooDeep` skip at that directory |
| `max_entries` | 200 000 | The walk stops; one `OutOfBudget` skip at the directory where it stopped |

Both are recorded when they bite. A directory holding two hundred thousand files
does not make SURE hang; it makes SURE say it stopped and where.

`max_entries` counts every directory entry the walk *meets*, including ones it
then declines, because the work of meeting them has already been done. That is
why the budget is a bound on SURE's effort and not a promise about the size of
the project.

The depth limit is checked twice — once by the parent before recursing and once
at the top of `visit` — and there is a test for the second. A limit that holds
only because every caller remembered to check it is not a limit.

### 3. It says what it did not look at

`Scan::entries()` is what was found. `Scan::skipped()` is what was not, and it is
a value in the result rather than a log line. Both kinds of skip are in it, and
`Scan::is_complete()` is the one question a caller must answer before treating
the first as "the project":

```rust
let scan = scan(root, ScanOptions::default())?;
if !scan.is_complete() {
    for loss in scan.losses() {
        eprintln!("{}", loss.plain_description());
    }
}
```

`is_complete()` is `false` when anything was lost. It is **`true`** when the only
skips are the declared scope — `.git`, `node_modules`, `target/`, caches — because
leaving those out is what the scan said it would do, and calling that a loss would
make "complete" mean "no skips at all", which is a bar no real project meets and
therefore a check nobody would read.

## The skip vocabulary

Ten reasons, in `crates/sure-core/src/scan/skip.rs`. Each answers two independent
questions with a **full `match`**:

| Reason | `is_by_design` | `loses_coverage` | What it is |
| --- | --- | --- | --- |
| `VersionControl` | ✔ | | `.git`, `.hg`, `.svn`, `.bzr`, `.jj` |
| `Vendored` | ✔ | | `node_modules`, `vendor`, `.venv`, … |
| `BuildOutput` | ✔ | | `target`, `dist`, `build`, `.next`, … |
| `Cache` | ✔ | | `__pycache__`, `.pytest_cache`, `.DS_Store`, … |
| `SureCache` | ✔ | | `.sure` — see below |
| `TooDeep` | | ✔ | Deeper than `max_depth` |
| `OutOfBudget` | | ✔ | The entry limit was reached |
| `NotFollowed` | | ✔ | A symlink, junction or other reparse point |
| `Unreadable` | | ✔ | The operating system would not list it, or would not say what it is |
| `SpecialFile` | | ✔ | A socket, device or pipe — not a regular file or directory |

The two predicates are deliberately **not** negations of each other. `loses_coverage`
is not `!is_by_design()`. A negation answers `false` — "not a loss" — for a variant
nobody thought about, and the quiet direction is the one that produces a false
green. Written as two full matches, a new variant stops both from compiling until
someone answers both questions, on purpose. The general rule is recorded in
`docs/architecture/FROZEN_SEMANTICS.md` § Closed vocabularies are matched in full.

Three of these deserve their reasoning written down:

**`SureCache` is not like the other caches.** A fingerprint taken over a tree that
contains SURE's own output changes when SURE runs. Checking a project would then
change the thing being checked, and the second check would be of a different
project state from the first — which is the definition of an unfalsifiable check.
The name comes from `sure_core::paths::PROJECT_CACHE_DIR` rather than being
spelled `.sure` twice, so the directory SURE writes into and the directory SURE
skips cannot drift apart.

**`NotFollowed` is a loss even though not following is deliberate.** What the link
points at is not in the scan. Classifying it as "by design" because the scanner
chose it would be the scanner grading its own decision, which is the category of
reasoning this whole product exists to reject. The same applies to `SpecialFile`:
a named pipe is an entry that exists and is not in the list.

**`OutOfBudget` is a partial account by nature.** The walk stopped, so nothing
past that point is in the scan and no skip was recorded for it either. The skip
list is therefore not a complete inventory of what was missed — which is exactly
why `is_complete()` exists rather than a caller adding up skip counts.

`Scan::skip_counts()` returns one row per reason, **including the ones with a
count of zero**. A table that is missing rows makes a reader wonder what was
omitted; a table with a zero in it says the scan considered that category and
found nothing.

## The ignore tables

`crates/sure-core/src/scan/ignore.rs` holds two tables of exact names:

- `IGNORED_DIRECTORIES` — names that are never a directory a project keeps its own
  work in.
- `IGNORED_FILES` — names that are never a *file* a project keeps its own work in.

### Why files and directories are separate

`build` is a directory of compiled output, and `build` is a shell script at the
root of a great many projects — one a person wrote by hand and would be right to
expect SURE to read. So are `target` and `dist`. A single table keyed on the name
alone cannot tell those apart, and the version that cannot tell them apart drops a
file that is genuinely part of the project. The two tables are separate because
that is the honest shape of the knowledge: *this name is never a directory a
project keeps its work in* and *this name is never a file a project keeps its work
in* are two different claims.

`IGNORED_FILES` is short on purpose. A file is a thing a person may have written
and named by hand, so a name only qualifies if some *other* program creates it
without being asked. It holds `.git` (a file rather than a directory in a
submodule or a linked worktree), `.DS_Store`, `Thumbs.db` and `desktop.ini`. The
last three are there so that one developer's machine does not change what the
project looks like to everybody else — which is what a fingerprint has to be
stable against.

### What is deliberately left out

`bin`, `obj`, `out`, `Debug`, `Release` and `third_party` are output or vendor
directory names for some toolchain, and ordinary directory names for others.
`bin/` holds hand-written scripts in a great many Node and Python projects;
`third_party/` is usually vendored source the project is answerable for. Leaving
a real directory out of a scan is a loss, so a name that could plausibly be
project content stays in the scan.

`dist`, `build` and `target` **are** in the directory table, which is the opposite
call, and the difference is frequency rather than principle: a project that keeps
its own source in `build/` is rare enough that "SURE did not look inside build/"
is a better failure than silently reading a stale output tree. A reader who
disagrees can see the list and say so, which is the point of it being one readable
table instead of a rule spread through the walker.

### Matching is by exact name, not by pattern

A glob language (`**/target/**`, `*.pyc`) is more expressive and much harder to be
sure about. The question these tables answer is narrow, and a name that cannot
answer it plainly does not belong in a table. Patterns arrive when something needs
them, with the evidence that it does.

### Case

A name is compared under the platform's case rule, and `matching_rule` takes that
rule as an **argument** rather than reading it from the platform, so both rules can
be tested on one machine. On Windows there is no `NODE_MODULES` distinct from
`node_modules`, so it is the same directory and the same answer; on Linux there
is, and a directory called `NODE_MODULES` there is a directory the project made.

Folding is ASCII-only. Every name in both tables is ASCII, so a non-ASCII name
cannot become one by being folded — and the full Unicode folding rules are
locale-dependent in a way that would make the answer depend on the machine. `İ`
alone lowercases to one code point in some locales and two in others.

### The root is not subject to the tables

A scan of a directory named `target` scans it. The tables describe what is inside
a project, and the caller has named this directory as the project. Applying them
at the root would mean a user could point SURE at a checkout and be told the
project is empty.

## The four refusals

Some things are not a scan with losses; they are no scan at all. Returning an
empty `Scan` for any of them would be the worst available answer, because an empty
scan and a project with nothing in it look the same.

| `ScanError` | When | Why it is not an empty scan |
| --- | --- | --- |
| `NotAbsolute` | The root is relative | Resolving it against the current directory would make what SURE read depend on where it was started, and every path in the result relative to a directory nobody named |
| `Missing` | Nothing at the root | A typo would be reported as a project with no files |
| `NotADirectory` | The root is a file | Reading the file and calling it a project |
| `Unreadable` | The root cannot be listed | There is nothing to report, and saying "no findings" would be false |

Each message names the path, says what SURE did instead of continuing, and quotes
the operating system verbatim when the operating system is the one that said no —
`Access is denied. (os error 5)` tells a user to fix a permission and
"could not be listed" does not.

## Determinism

Entries come out in a fixed order: each directory's children are sorted by name
and descended into as they are reached. The sort is by the `OsString` the operating
system gave, **not** by its rendered text. A fingerprint taken over a list whose
order moves is a fingerprint that changes when nothing did.

The two orders are not the same order, and there is exactly one case where they
differ: a name that cannot be rendered faithfully. An unpaired surrogate on
Windows and an invalid byte sequence on Unix both render as U+FFFD, which is
greater than every code point below U+FFFD — so as text such a name sorts after a
name it sorts before as a name. Those are the only two names whose text and whose
bytes disagree, and
`a_name_that_is_not_valid_unicode_is_ordered_by_the_name_and_not_by_its_text`
builds exactly that pair. For two names that both render faithfully the two orders
agree, which is why nothing else in the suite can see the difference and why the
test has to reach for an unrenderable name.

Names are stored as `PathBuf`/`OsString` throughout and converted to text only at
the point of display, by `display_path`, which joins components with `/` on every
platform. `Display for Path` gives backslashes on Windows, and a project stored in
a document has to compare equal for two people on two platforms.

`display_path` is a **reporting** loss for a component that is not valid Unicode:
it becomes U+FFFD. That is stated rather than hidden, and `Entry::path` is the real
path, so anything that has to tell two files apart uses that and not the text. In
the test above the two entries render identically and are still two entries.

## What discovery does not do

**It reads no file contents.** Nothing in `src/scan/` calls `File::open`,
`fs::read`, `fs::read_to_string`, `BufReader`, `read_link` or `canonicalize`, and
`crates/sure-core/tests/scan_project.rs` asserts exactly that against the source
of the four files. This is not a tidiness rule: a scan of a project whose files
are enormous, encrypted, on a slow network share, or cloud placeholders that would
be *fetched* by being read, costs the same as a scan of any other. Deciding what
to read, and reading it, belongs to the fingerprint (P2-T002, P2-T003) and the
readers after it.

**It does nothing about `.gitignore`.** A project's own ignore file is a statement
about what belongs in the *repository*, which is a different question from what
belongs in a *check*. A test fixture deliberately kept out of a repository is still
worth reading, and a file the project forgot to ignore is not made uninteresting by
being tracked. Reading those rules is a decision for a task that needs it, with its
own evidence.

**It does not decide what anything is.** Finding `package.json` here does not mean
the project is a Node project. That is P2-T004 onwards.

**It does not record file metadata.** No size, no modification time, no contents —
only the name and whether it is a file or a directory. The fingerprint's job is to
decide which metadata is part of the project's identity.

## Known coverage gaps

Each is recorded so it is not forgotten. None is resolved by a task yet. The
first two are unconstructible inputs; the third is a platform question.

1. **A directory the operating system refuses to list.** The arm that handles this
   is real and is tested, but not against a genuinely unreadable directory: making
   one needs either a privileged user or an ACL change, and `chmod 000` does
   nothing when the tests run as root. A test that passes because its premise did
   not hold is worse than no test, so the tests reach the arm two other ways — a
   path that does not exist, and a path with an interior NUL byte, which is refused
   before any system call. What they do **not** establish is that the operating
   system produces that error for a real unreadable directory.

2. **A single entry the directory could not describe.** The `Err` arm of iterating
   `read_dir`'s listing, where the directory is readable but one of its entries is
   not. Reaching it needs an entry the operating system fails to describe, which
   cannot be built. The **mutation** of this arm — dropping the entry instead of
   recording it — is the one mutation of twenty-eight that the suite does not
   catch, and it is recorded here rather than removed from the mutation list: a
   mutation that is not caught because its input cannot exist is a gap, and a
   mutation quietly deleted from the list is a gap nobody knows about.

3. **macOS and a name that is not valid Unicode.** The order test for such a name
   is `#[cfg(not(target_os = "macos"))]`, because APFS validates a file name as
   UTF-8 and the fixture may not be constructible there. That is a *belief* about
   APFS, not something verified from a Windows machine, and it is written here as
   a belief. If it is wrong, the cost is that macOS does not run a test it could
   have run, and the fix is to drop the `cfg`.

The first two are stated in the module comment of
`crates/sure-core/tests/scan_project.rs`, which is where a reader looks for what a
test file does *not* cover.

## Enforced by

| Statement | Where the meaning lives | Enforced by |
| --- | --- | --- |
| The scan stays inside its root | this document, `crates/sure-core/src/scan/mod.rs` | `a_relative_path_stays_relative_all_the_way_down`, `a_link_is_recorded_and_never_followed` |
| A link is never followed | this document | `crates/sure-core/tests/scan_project.rs` (in/out/at-root link cases) |
| The walk is bounded by depth and entries | this document, `ScanOptions` | `the_depth_limit_stops_the_walk_and_says_where`, `the_entry_limit_stops_the_walk_and_is_recorded_once`, `a_limit_that_is_never_reached_is_never_reported` |
| A skip is a value, never silence | this document, `crates/sure-core/src/scan/skip.rs` | `Scan::skipped`, `Scan::skip_counts` |
| Each reason answers both questions, in full | this document | `every_reason_answers_both_questions_consistently` |
| VCS/vendor/build/cache are skipped and reported | `tasks/tasks.json` P2-T001 | `the_four_categories_the_task_names_are_left_out_and_each_one_is_reported` |
| Only file names are never project content | this document | `a_name_that_is_both_a_tool_output_and_a_script_is_only_ignored_as_a_directory` |
| Case folding is ASCII and platform-dependent | this document | `case_is_folded_only_where_the_platform_folds_it`, `case_folding_does_not_reach_a_name_that_is_not_ascii` |
| Order is fixed and independent of the filesystem | this document | `a_small_project_comes_back_as_files_and_directories_in_a_fixed_order`, `the_order_does_not_depend_on_the_order_the_filesystem_returns` |
| Order is by the name, not by its rendered text | this document | `a_name_that_is_not_valid_unicode_is_ordered_by_the_name_and_not_by_its_text` |
| An ignored name is left out at any depth | this document | `a_left_out_directory_is_left_out_at_any_depth`, `a_nested_version_control_directory_is_left_out_too` |
| A link is never filed under the name it happens to have | this document | `a_link_is_a_loss_whatever_it_is_called` |
| The fold is the ASCII one | this document | `the_fold_is_ascii_rather_than_the_one_unicode_defines` |
| Discovery reads no file contents | this document | `scanning_the_repository_does_not_open_any_file` |
| A refusal is never an empty scan | this document | `crates/sure-core/src/scan/error.rs`, the four refusal tests in `scan_project.rs` |

## Related decisions

| ADR | Topic |
| --- | --- |
| 0001 | Rust local core and crate boundaries — `scan` is in `sure-core`, not in a crate of its own |
| 0002 | Local-first privacy — a scan tells nobody anything and sends nothing anywhere |
| 0007 | Windows primary development — junctions, long paths and case-insensitivity are primary cases, not edge cases |
| 0010 | Frozen domain semantics live in code — the skip vocabulary is part of what a report means |
| 0012 | Diagnostics are records, not a logging framework — which is why a skip is a field on the result and not a log line |

[`SkipReason`]: ../../crates/sure-core/src/scan/skip.rs
[`SkipReason::NotFollowed`]: ../../crates/sure-core/src/scan/skip.rs
[`ScanOptions`]: ../../crates/sure-core/src/scan/mod.rs

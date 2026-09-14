# Fingerprinting

Fingerprinting is step 3 of `docs/architecture/CHECK_PIPELINE.md`: naming exactly
the project state that evidence is about. It is implemented by
`sure_core::fingerprint` (`crates/sure-core/src/fingerprint/`).

The whole of it is one question, asked once and answered by everything
downstream: **is the evidence I have still about the project in front of me?** A
result carries the fingerprint it was produced from; a later run computes one;
`ProjectFingerprint::matches` answers yes or no. Nothing else in SURE compares
files, and nothing else decides whether an old green is still a green.

That gives both ways this can be wrong a name, and they are not equally bad.

| | What happens | What it costs |
| --- | --- | --- |
| **The fingerprint moves when the project did not** | a true result is marked stale | work done again — and a person who sees it often enough stops reading the word "stale" |
| **The fingerprint stays when the project did change** | a result from one project is reported as current for another | a green that outlived its evidence — the first line of `CLAUDE.md` |

So every decision that could go either way goes the way that moves the
fingerprint. That is not caution for its own sake: the second row is the product
failure, and the first row's cost is paid in attention, which is the thing the
second row needs.

## Coverage: the rule

**A file is part of the fingerprint if and only if a check could read it.**

A check reads what the scan found, so the fingerprint covers the scan's files and
no others — the same ignore tables, applied by the same comparison
(`crate::scan::ignore::left_out`), under the same options. Concretely:

| Path | In the fingerprint? | Why |
| --- | --- | --- |
| a tracked file that differs from HEAD | yes | the scan reads it, and Git says it moved |
| a tracked file with no changes | no | the scan reads it, but nothing about it changed and HEAD already names its content |
| an untracked file the ignore tables keep | yes | the scan reads it and Git has never seen it |
| an untracked file under `node_modules/` | no | no check reads inside it |
| a **tracked** file under `target/` | no | no check reads inside it — see below |
| a nested repository Git reports as one untracked directory | yes, by walking it | the scan walks it, so a check reads those files |
| **a link** | yes, by its target | no check reads through a link — see below |
| anything under `.sure/` | no | see below |
| a file outside the project, in the same repository | no | SURE was asked about this project |

Two of those rows are decisions rather than consequences, and both were argued
rather than assumed.

### A tracked file under a build directory is not covered

It is in the repository, so it belongs to the project. It is also a file no check
will ever open, so changing it cannot change any answer SURE gives. Including it
would mark every result stale whenever a committed artefact was rebuilt — the
first failure in the table above, paid for nothing. The rule as stated resolves
it without a special case, and
`a_tracked_file_in_a_build_directory_is_not_in_the_fingerprint` pins it so that
it stays a decision.

The cost is real and worth stating: an edit to such a file with no other change
leaves the fingerprint alone, so a result produced before the edit is reported as
current. It is current — for everything a check can see.

### A link is covered, and it is the one case the rule does not decide

A link is recorded as the path it points at, and nothing reads through it. That
last part is the whole of the scan's behaviour too — a link is
`SkipReason::NotFollowed` — so a file reachable only through a link is a file no
check can read, and leaving it out follows the rule rather than departing from
it. A change behind an unchanged link therefore does not move the fingerprint,
which is gap 2 below.

What does depart from the rule is the link **itself**. No check reads a link, so
by the rule as written it should not be in the fingerprint — and it is. A tracked
link is part of the repository, and retargeting one changes what the project
contains while leaving every file it names untouched. Of the two failures in the
table at the top, missing that one is the worse, so a tracked link counts.

This is the **only** place the fingerprint covers something a check cannot read,
and it is the only place `crates/sure-core/src/fingerprint/git/mod.rs` calls
itself wider than the scan rather than equal to it. It is named here so that the
rule above is understood as having one argued exception rather than none.

### SURE's own output is not covered

`.sure/` is in `IGNORED_DIRECTORIES` alongside the build directories, for a
reason none of the others have (see `SkipReason::SureCache`). A fingerprint taken
over a tree containing SURE's own output would change when SURE ran, so checking
a project would change the thing being checked, and the second run's fingerprint
would never equal the first's. Every stored result would be stale the moment it
was written.

## What it does not claim

The rule is about **coverage** — which files the fingerprint is over. It is not a
claim that a check read the file, only that it could.

The stronger property — a fingerprint that changes only when a check has actually
read something — would need every check to declare its inputs up front, which
`docs/architecture/EVIDENCE_MODEL.md` leaves to the tasks that build them. Until
then, a change to a covered file that no check happened to read moves the
fingerprint, and that is the deliberate direction of error.

## The two kinds

| Kind | When | Where |
| --- | --- | --- |
| `FingerprintKind::Git` | the project is in a working tree | `sure_core::fingerprint::git_fingerprint`, P2-T002 |
| `FingerprintKind::Content` | the project is not under version control | P2-T003 |

There is deliberately no function that picks between them. Choosing by looking at
the project is a decision with a wrong answer — a project inside a repository
that is not the project is exactly the case where the obvious check is wrong, and
the wrong answer is a fingerprint of somebody else's state. P2-T003 owns making
that decision with its own evidence.

`ProjectFingerprint::matches` compares `kind` and `digest`, never `id`. The `id`
is fresh on every computation so that two results can be told apart by value; the
`digest` is what two states share when they are the same state.

## The Git fingerprint

### What Git is asked

One invocation, through one abstraction (`Git`, and `Git::STATUS_ARGUMENTS` is
read by a test that fails if a second place starts Git):

```
git --no-optional-locks -C <root> status --porcelain=v2 -z --branch \
    --untracked-files=all --no-renames -- .
```

plus `git --no-optional-locks -C <root> rev-parse --show-prefix`, which is also
how "there is no repository here" is detected.

| Flag | Why it is there |
| --- | --- |
| `--porcelain=v2` | the format Git documents as stable for programs, rather than one that changes with configuration |
| `-z` | no quoting: a path with a space, quote, backslash or newline arrives as itself |
| `--branch` | the commit and the branch in the same output, so there is no second moment to read |
| `--untracked-files=all` | every untracked file; `normal` would name a directory and leave its contents unaccounted for |
| `--no-renames` | a similarity heuristic whose threshold is configuration and whose behaviour has changed between Git versions cannot be allowed to decide a digest |
| `-- .` | only this project, in a repository that may hold several |
| `--no-optional-locks` | `status` must not refresh the index: checking a project must not change it |

### `--relative` is not used, and must not be

It reads like exactly the right flag: paths relative to the current directory,
changes outside it excluded — which is what `-- .` plus `--show-prefix` achieve
between them. On Git 2.55.0 it produced **no output at all** for a working tree
with changes in it, silently, with a successful exit. SURE would have read that
as "this project is clean" and fingerprinted it as such.

It was found by running the real Git rather than by reading its documentation,
and it is the reason `the_status_arguments_are_the_ones_the_module_doc_explains`
asserts the flag's **absence**: the next person to read the documentation will
reach for it, and the test should stop them.

The prefix is taken off by hand instead, with `Path::strip_prefix` and not a
string comparison. A repository holding `app/` and `app-old/` defeats a string
prefix, and `app-old/file.rs` would come out of one as `-old/file.rs` — a file
that does not exist, digested as though it did.

### It reads the files, not Git's diff

The cheaper design hashes the text of `git diff`. It is wrong twice over. The
text depends on configuration and on Git's version — `diff.algorithm`,
`.gitattributes` filters, rename thresholds — so one project state can produce
two digests, and a fingerprint that moves when nothing did is one nobody
trusts. And a diff describes a change *from HEAD*, which says nothing at all
about a file the repository does not track.

Git decides **which** files matter. The bytes decide **what** is in them.

### The walk inside a fingerprint

One record in Git's status can name a directory rather than a file, and there is
exactly one case in practice: a nested checkout. Git reports it as a single
untracked directory and does not look inside — it cannot, it is somebody else's
repository — so SURE is the one that decides what is in there.

It decides with **the same scan a check uses**, under the same options, which is
what keeps the coverage rule true rather than slogan-shaped: the files inside are
files a check can read, so they are in the fingerprint. Each contributes its path
*and* its contents, because a file that moved without changing is still a
different project state.

A walk that comes back incomplete is an error and not a shorter digest. The files
it did not reach are files a check would still read, so hashing the part that
fitted would claim coverage nothing looked at — and the fingerprint would then
call an old result current for a change inside the part that was skipped.

### The digest

SHA-256, through the `sha2` crate, with every field length-prefixed and the whole
thing opened by a domain tag, `sure.git-fingerprint.v1`. The three reasons:

- **Not `DefaultHasher`.** It is documented as not stable across Rust releases.
  A fingerprint that changes when the toolchain is upgraded marks good evidence
  stale, and one that collides marks stale evidence fresh.
- **Cryptographic, not a fast non-cryptographic hash.** Two project states
  sharing a digest is exactly the false green this product exists to prevent, and
  an adversary choosing the collision is in scope: the project being checked is
  written by a coding agent.
- **Length-prefixed fields.** Without it, `("ab", "c")` and `("a", "bc")` are the
  same bytes into the hasher, and a project that moved one character across a
  file boundary would keep its fingerprint.

The two halves are digested separately (`dirty_digest`, `untracked_digest`), so a
report can say *what* changed rather than only *that* something did. `None` is
not the digest of an empty list: "this project is clean" and "this project has an
empty change list" are two states, and only one of them is ever true.

### Determinism

The same project state produces the same digest on every run, on every machine,
on every platform, and under every later version of SURE that still speaks the
domain tag. Four decisions exist only to keep that true:

1. **Paths are sorted by bytes before hashing**, because the order Git lists
   changes in is a property of Git and not of the project.
2. **Paths are written with `/` on every platform**, through
   `crate::scan::display_path`, so a checkout on Windows and one on Linux do not
   produce two digests for one state.
3. **The branch name is recorded and not digested.** Renaming a branch moves no
   file; a fingerprint that moved would teach a person to ignore the word stale.
4. **Limits produce an error, never a partial fingerprint.** `max_files` and
   `max_bytes` are limits on work, and reaching one is `TooManyFiles` or
   `TooManyBytes` — never a digest over the part that fitted, which is a wrong
   answer wearing the shape of a right one.

The cost of (3) is the flip side of digesting HEAD: see below.

### HEAD is digested, and that is deliberate

Two commits with byte-identical trees are the same project *content* and
different project *states*. A checkout of a different commit with no local
changes is not the state the evidence was produced against, so the fingerprints
differ — `two_different_commits_of_one_tree_are_two_fingerprints`.

This is the one place the design accepts the first failure in the table at the
top. `git checkout` of a branch at the same commit, or a rebase that rewrites
history without changing a file, moves the fingerprint and marks good evidence
stale. That is accepted because the alternative — trusting that the tree is the
only thing that matters — is a claim about what future checks will read, made
today, by a component that cannot know.

## Known coverage gaps

Each is recorded so it is not forgotten, with the reason it is not closed.

1. **A symbolic link on Windows.** `CreateSymbolicLinkW` needs Developer Mode or
   administrator rights, and the primary development machine has neither — this
   was probed, not assumed. The `Contents::Link` path is therefore tested on Unix
   only. Windows is not left unexamined, though: Git's own answer to the same
   problem is `core.symlinks=false`, under which a link is checked out as an
   ordinary file holding its target, so on such a machine there is no link in the
   working tree for SURE to find and hashing what is there is correct. What is
   *not* established is the behaviour of a real link on a Windows machine with
   Developer Mode on. If it differs, the Unix test is where the difference shows
   up.

2. **A change behind an unchanged link.** A link contributes the path it points
   at and nothing else, so a file reached only through a link can change without
   the fingerprint moving. Reading through the link would read whatever it names,
   including outside the project, which every other part of SURE refuses to do.
   `a_change_behind_an_unchanged_link_is_not_a_change` asserts the current
   behaviour so that a change to it is a change somebody made on purpose. It is
   **not** a departure from the rule above: a check cannot read through a link
   either, so such a file is outside the rule to begin with. Recorded here
   because a reader coming from the link row in the table will want to know where
   the account of it stops — and because this behaviour is exactly as untested on
   Windows as gap 1 is.

3. **`assume-unchanged` and `skip-worktree`.** A file marked either way is
   reported by Git as unchanged, and SURE believes it. Both are instructions a
   person gives Git to lie about a file, deliberate and rare. Detecting them
   would mean a second Git invocation per fingerprint for a case that is already
   a person saying "ignore this".

4. **A file that cannot be read.** The `Unreadable` arm is real and is tested on
   Unix, where `chmod 000` produces it. It is not tested on Windows: there is no
   mode that stops the owner reading their own file, and a test that skips itself
   because its premise did not hold is worse than no test. The Windows arm is the
   same code path with a different `io::ErrorKind`.

5. **Sub-second races.** A file modified between the `git status` and the read
   of its bytes is digested as it was when read, and Git's record of *which*
   files changed is from the moment it ran. A change made in that window is
   accounted for on the next run. Closing it would mean holding a lock over the
   whole project, which costs more than it is worth for a window of
   milliseconds.

Gaps 1, 3 and 4 are stated in the module comment of
`crates/sure-core/tests/fingerprint_git.rs`, which is where a reader looks for
what a test file does *not* cover; that comment also names a fourth untested
case, `core.symlinks=false`, which is a branch of gap 1 rather than a separate
entry here. Gap 2's tests are themselves Unix-only, since producing a link on
Windows is the thing gap 1 says this machine cannot do — so on Windows the whole
of the link behaviour, wider and blind alike, is asserted by no test at all, and
this document is the only place it is written down. Gap 5 is a property of any
design that reads files after asking Git about them, and no test can pin it.

The same asymmetry runs through the "Enforced by" table below: the two rows
marked (Unix) are the ones with no Windows test, and they are the two rows about
links.

## Enforced by

| Statement | Where the meaning lives | Enforced by |
| --- | --- | --- |
| The same state gives the same digest | this document | `the_same_project_state_fingerprints_to_the_same_value_every_time`, `two_checkouts_of_the_same_state_agree` |
| HEAD is part of the fingerprint | this document | `a_different_commit_is_a_different_fingerprint`, `two_different_commits_of_one_tree_are_two_fingerprints` |
| An edited tracked file is a change | `tasks/tasks.json` P2-T002 | `an_edited_tracked_file_is_a_different_fingerprint` |
| An untracked file is a change | `tasks/tasks.json` P2-T002 | `a_new_untracked_file_is_a_different_fingerprint` |
| A deletion is a change and not an empty file | this document | `a_deleted_tracked_file_is_a_different_fingerprint`, `a_deleted_file_and_an_empty_file_are_two_states` |
| Content, not modification time | this document | `an_untracked_file_that_is_rewritten_with_the_same_bytes_is_not_a_change`, `a_file_put_back_the_way_it_was_is_the_same_fingerprint` |
| The two halves are told apart | this document | `the_two_halves_of_a_dirty_tree_are_digested_separately` |
| Installed dependencies are not project content | this document | `a_change_inside_a_vendor_directory_is_not_a_change` |
| A tracked file in a build directory is not covered | this document | `a_tracked_file_in_a_build_directory_is_not_in_the_fingerprint` |
| SURE's own output is not covered | this document | `sure_own_cache_inside_the_project_does_not_change_the_fingerprint` |
| `.gitignore` is honoured | this document | `an_untracked_file_that_git_ignores_is_not_a_change` |
| A branch rename is not a project change | this document | `renaming_the_branch_is_not_a_change_to_the_project` |
| Spaces and non-ASCII paths arrive whole | `RUST_DESIGN.md` §Git | `a_path_with_spaces_and_non_ascii_characters_is_fingerprinted_like_any_other`, `a_leading_space_is_part_of_a_name` |
| The case rule reaches the fingerprint | `RUST_DESIGN.md` §Git | `a_name_that_differs_only_in_case_is_a_build_directory_on_one_platform_and_not_the_other` |
| A path may be longer than the old Windows limit | `RUST_DESIGN.md` §Git | `a_path_longer_than_the_old_windows_limit_is_fingerprinted` |
| A project in a repository is fingerprinted as itself | this document | `two_projects_in_one_repository_do_not_change_each_other`, `a_change_inside_a_project_that_is_not_the_repository_root_is_read` |
| A directory Git will not descend into is walked and its files read | this document | `a_directory_git_will_not_descend_into_is_walked_and_read` |
| A file's path is digested beside its contents | this document | `a_directory_git_will_not_descend_into_is_walked_and_read` (the move at the end) |
| A gitlink is a directory whether or not it exists | this document | `a_submodule_that_was_never_checked_out_is_a_directory_and_not_a_missing_file` |
| A link is recorded by its target | this document | `a_link_is_recorded_by_its_target_and_not_by_what_it_points_at` (Unix) |
| A change behind a link is not seen | gap 2 above | `a_change_behind_an_unchanged_link_is_not_a_change` (Unix) |
| A limit is an error, never a partial fingerprint | this document | `a_project_with_more_changes_than_the_limit_has_no_fingerprint`, `a_project_with_more_content_than_the_limit_has_no_fingerprint`, `the_byte_limit_is_over_the_fingerprint_and_not_over_each_file` |
| A walk that lost something has no fingerprint | this document | `a_walk_that_lost_something_has_no_fingerprint` |
| A refusal is never an empty fingerprint | this document | `a_directory_that_is_not_in_a_repository_has_no_fingerprint`, `git_that_will_not_start_is_a_refusal_and_not_an_empty_fingerprint`, `a_relative_root_is_refused_rather_than_resolved_against_the_current_directory`, `a_change_to_a_file_sure_cannot_read_has_no_fingerprint` (Unix) |
| Git is started in exactly one place | `RUST_DESIGN.md` §Git | `git_is_started_in_exactly_one_place` |
| `--relative` is never used | this document | `the_status_arguments_are_the_ones_the_module_doc_explains` |
| The digest is a correct SHA-256 | this document | `the_published_sha256_vectors_come_out_right` |
| Fields cannot be re-split across a boundary | this document | `the_framing_separates_a_split_in_a_different_place` |
| A Git digest is never a content digest | this document | `a_digest_of_one_kind_is_never_a_digest_of_another` |
| An absent half is not an empty one | this document | `an_absent_field_is_not_an_empty_one` |

## Related decisions

| ADR | Topic |
| --- | --- |
| 0001 | Rust local core and crate boundaries — the fingerprint is in `sure-core` |
| 0002 | Local-first privacy — a fingerprint reads the project and sends nothing |
| 0005 | Evidence over claims — a fingerprint is what makes evidence expirable |
| 0007 | Windows primary development — case, long paths and links are primary cases |
| 0010 | Frozen domain semantics live in code — `GitState` and `FingerprintKind` are part of what a result means |
| 0012 | Diagnostics are records, not a logging framework — a fingerprint is a value on a result, not a log line |

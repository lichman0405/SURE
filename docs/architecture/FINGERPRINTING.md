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
| `FingerprintKind::Git` | the project is at the root of its own working tree | `sure_core::fingerprint::git_fingerprint` (P2-T002) |
| `FingerprintKind::Content` | anything else | `sure_core::fingerprint::content_fingerprint` (P2-T003) |

### Which one a project gets, and why "is it in a repository" is the wrong test

`sure_core::fingerprint::project_fingerprint` is the one function that chooses.
The rule it applies is not "is this directory inside a repository" but **"is this
directory at the root of the working tree that contains it"** — Git reports a
path from the top of the working tree to where the question was asked
(`rev-parse --show-prefix`), and that path is empty exactly when the two are the
same directory.

The obvious test answers `yes` for a directory one component deep in somebody
else's checkout, and everything Git would then contribute is about the *other*
project. `HEAD` is the outer repository's — the Git fingerprint digests it on
purpose, because a checkout of a different commit is a different state — so a
commit anywhere else in the repository moves the subdirectory's fingerprint
although not one of its files changed. Evidence would be marked stale over and
over for a project nobody touched, which is how a person learns to stop reading
the word "stale", and the failure that teaches them to ignore is the dangerous
one. `-- .` and `--show-prefix` scope the *change list* correctly; what they
cannot do is make the repository the project's.

A content manifest has none of that: it reads the project and only the project,
so nothing outside can move it. It costs more, and that is the deliberate
direction.

This is measured rather than argued, in
`the_git_kind_moves_for_a_commit_the_project_is_not_part_of_and_the_content_kind_does_not`
— which asserts both halves, so that neither can quietly stop being true and
leave the reason looking like folklore.

### A machine without Git is an error, not a content fingerprint

`FingerprintError::GitUnavailable` is **not** turned into a fallback. The kind a
project gets has to be a function of the project and not of the machine: falling
back would mean one unchanged directory produced a `Git` fingerprint on a
developer's laptop and a `Content` fingerprint on a build agent without Git, and
those two values are not equal — `matches` compares `kind`. Every stored result
would be reported stale on the other machine, and a project checked in both
places would never agree with itself.

A caller who knows the project is not a repository, or wants the manifest for a
reason of their own, calls `content_fingerprint` directly. That escape hatch is
explicit on purpose: it is a decision, so somebody makes it.

`ProjectFingerprint::matches` compares `kind` and `digest`, never `id`. The `id`
is fresh on every computation so that two results can be told apart by value; the
`digest` is what two states share when they are the same state.

## The Git fingerprint

### What Git is asked

Three invocations, all through one abstraction (`Git`; `Git::STATUS_ARGUMENTS`,
`Git::SAFETY_ARGUMENTS` and `Git::CONFIG_ARGUMENTS` are read by tests, one of
which fails if a second place starts Git). In the order they are made:

```
git --no-optional-locks -c core.fsmonitor=false --no-pager -C <root> \
    rev-parse --show-prefix
```

which is also how "there is no repository here" is detected;

```
GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=<root>/.git/config/sure-no-global-configuration \
    git --no-optional-locks -c core.fsmonitor=false --no-pager -C <root> \
    config --list --includes -z
```

which asks whether the project names a program, and refuses if it does; and

```
git --no-optional-locks -c core.fsmonitor=false --no-pager -C <root> status \
    --porcelain=v2 -z --branch --untracked-files=all --no-renames -- .
```

The order is not incidental. `rev-parse` is measured not to run a filter, so it
is safe before the check. `status` is measured **to** run one, so the check has
to come before it — a check that came after would be a report.

| Flag | Why it is there |
| --- | --- |
| `--porcelain=v2` | the format Git documents as stable for programs, rather than one that changes with configuration |
| `-z` | no quoting: a path with a space, quote, backslash or newline arrives as itself |
| `--branch` | the commit and the branch in the same output, so there is no second moment to read |
| `--untracked-files=all` | every untracked file; `normal` would name a directory and leave its contents unaccounted for |
| `--no-renames` | a similarity heuristic whose threshold is configuration and whose behaviour has changed between Git versions cannot be allowed to decide a digest |
| `-- .` | only this project, in a repository that may hold several |
| `--no-optional-locks` | `status` must not refresh the index: checking a project must not change it |
| `-c core.fsmonitor=false` | a repository's own configuration names a hook; see below |
| `--no-pager` | and names a pager, which is the same hazard one step removed |

### A repository is not allowed to make Git run a program

A project is untrusted input by assumption here — it may have been written by a
coding agent, or cloned from anywhere — and a repository carries configuration
that Git reads and obeys. Some of that configuration names **programs**, and
where it does, fingerprinting a project would be the same act as running that
project's code. SURE's premise is that it inspects a project without executing
it, so every route of that kind has to be closed or refused.

**The earlier version of this section was wrong, and this is the correction.**
It said the flags below bought the guarantee. They do not. A review of the commit
that added it found the route they miss, and the claim is retracted here rather
than quietly narrowed.

#### The routes, measured rather than reasoned about

Every row was established by running Git 2.55.0 with a marker program and
checking whether the marker appeared, with a positive control in each case — a
probe that reports `RAN` for everything is a probe that measured its own fixture
setup. `target/tmp/filter_probe.sh` is the script, kept out of the repository
because its job is done but not deleted because the next reader will want it.

| Route | Closed by | Measured |
| --- | --- | --- |
| `core.fsmonitor` hook | `-c core.fsmonitor=false` | the control fires the hook, the flag stops it |
| `core.pager` | `--no-pager` | Git does not page into a pipe, so this changes nothing today; passed so it cannot start to matter if this output ever stops being one |
| `post-index-change` hook | `--no-optional-locks` | the control fires the hook, the flag stops it, because the index refresh it depends on does not happen |
| `diff.<n>.textconv` | nothing — out of reach, and measured not to run | `status` does not run it |
| `git rev-parse` in general | nothing — measured not to run one | the prefix invocation does not run a filter |
| **`filter.<n>.clean\|smudge\|process`, defined by the repository** | **the repository is refused** | **the control fires it during a plain `git status` with nothing modified** |
| `filter.<n>.*` defined by the *machine*, named by a `.gitattributes` the repository carries | nothing — left open on purpose | `filter_probe.sh`, then `boundary_probe.sh` when the first version of this document implied it was closed; see "What is deliberately not covered" |

The second-to-last row is the one the earlier version missed, and it is the one
that cannot be closed from the command line. The last row is a second boundary
found while checking the fix, and it is a different thing: the project can reach
a program there, but cannot *choose* it.

#### Why a content filter cannot be overridden, only refused

It comes in two halves, and **both are the project's**. A tracked
`.gitattributes` names a driver — `*.psd filter=lfs` — and the command is a
`filter.<name>.clean` setting in the repository's own configuration. The name is
therefore known only after Git has read the project, so there is no fixed
`-c filter.<name>.clean=` to pass.

There is also no switch that turns in-tree `.gitattributes` off. That was
checked rather than assumed: `git help --config` lists `core.attributesFile`,
which is the *global* file, and nothing that disables the tracked one.

Neutralising it anyway would be worse than refusing. With the filter disabled,
Git compares a file's raw bytes against a stored version that was written
*through* the filter, and reports every such file as modified. That fingerprint
is wrong — and a wrong fingerprint marks real evidence stale or stale evidence
current, which is the failure this product exists to prevent. So the answer is
`FingerprintError::RepositoryRunsPrograms`, and it names the settings it found.

#### What is deliberately not covered

**Only the repository's own configuration is refused** — `include.path` targets
and the worktree config included. A filter defined by the *machine's* own config
is not, and the reason is that it is the user's installed tooling rather than the
project's doing: refusing every repository on a developer machine with Git LFS
installed would make SURE useless on exactly the machines most likely to have it,
and would be refusing on the strength of a fact the project did not supply. The
boundary is the same one `EXECUTION_SAFETY.md` draws: what SURE declines to run
is what the *project* names.

**That boundary is narrower than it sounds, and the difference was measured.**
A project supplies half of the pair on its own: a tracked `.gitattributes` can
say `*.psd filter=lfs`, and Git then looks up `filter.lfs.clean` in whatever
configuration it reads — **including the machine's**. `target/tmp/boundary_probe.sh`
puts a machine-scope filter command behind a driver name the repository does not
define, and a plain `git status` runs it. On the machine this was measured on,
`C:/Program Files/Git/etc/gitconfig` and `~/.gitconfig` both carry
`filter.lfs.{clean,smudge,process}` and `git-lfs` is on the `PATH`, so this is a
live route here and not a hypothetical one.

It is left open, and the distinction that makes that defensible is **who chooses
the program**. The project cannot name one: it can only ask for a driver, and the
command comes from a configuration the user wrote. So the set of programs a
project can reach is exactly the set of filters already installed on the machine
— on this one, `git-lfs` and whatever else the user added. That is not
project-controlled executable code, which is the line `EXECUTION_SAFETY.md`
actually draws, and it is why this is recorded as a boundary rather than as a
defect. A reader who thinks the refusal is total should read this paragraph
twice: SURE can be made to run `git-lfs` by a repository, and cannot be made to
run anything the repository chose.

Closing it would mean reading the driver names out of the project's attributes —
`.gitattributes`, `$GIT_DIR/info/attributes` — and refusing when any of them
resolves in any scope. That is a real design decision with a real cost
(`git check-attr` over every path, and a refusal for every repository that uses
Git LFS legitimately), so it is named here to be decided deliberately and not
folded into a fix for something else. It is **not** the defect the review named:
that one let a *repository* choose the program, which is arbitrary code execution
by the project, and it is closed. This one cannot be made to run anything the
project chose. No task covers it yet — it is new — and it is raised in
`progress/HANDOFF.md` for the owner to promote or dismiss.

#### How the question is asked

One invocation, before the status and not after — the status is the invocation
that runs these, so a check that came afterwards would be a report rather than a
prevention:

```
GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=<root>/.git/config/sure-no-global-configuration \
    git config --list --includes -z
```

| Part | Why |
| --- | --- |
| `--list` | the driver name is the project's to choose, so the answer has to be found rather than guessed |
| `--includes` | **redundant today, kept on purpose.** A repository reaches a filter through `include.path` as easily as through its own file — but includes are followed whenever Git searches all its files, so with no scope named, `--list` already reads them. Measured: the included filter was found with the flag and without it. It stays so the property is stated in the invocation rather than inherited from a default, and so a scope added later cannot silently stop includes being read |
| `-z` | a setting's value is an arbitrary program with arbitrary arguments, including newlines; `-z` means reading the setting rather than parsing Git's escaping of it |
| `GIT_CONFIG_NOSYSTEM=1` | drops the system scope |
| `GIT_CONFIG_GLOBAL=…` | drops the global scope. The path is under `.git/config`, which is a **file** in every repository Git makes — including linked worktrees and submodules, where `.git` is a file — so nothing can exist at that path and no project can add settings to the answer about itself. It also needs no `/dev/null`, which is not a Windows name |

`--local` is *not* used, and the reason is measured: `--local --list` does not
follow `include.path` without `--includes`, and even with it misses
`.git/config.worktree`. Both are scopes the repository itself supplies. Naming
no scope at all gets all three. The `-z` record format is `key\nvalue\0`,
verified by counting separators rather than by reading the documentation.

**A correction on `--includes`.** The first version of this section said the
flag was load-bearing, on the reasoning that includes are off as soon as a scope
is named. That reasoning is right about `--local` and wrong about this
invocation: `--list` with **no** scope names already follows includes, so the
flag changes nothing here. It was measured after a mutation showed that dropping
it failed only the test pinning the flag and no behavioural test. The flag is
kept — stating the property beats inheriting it from a default — but the comment
that called it necessary was false and is corrected here, in the source, and in
the test that pins it.

#### A project cannot make Git wait for an answer either

`GIT_TERMINAL_PROMPT=0` and a null stdin mean a Git that wants to ask a question
fails instead of waiting for one nobody will give. These are on the process, not
on `SAFETY_ARGUMENTS`, and they are the other half of the same rule: a check must
not be stoppable by the thing it checks, and a check that never returns cannot be
told apart from one that is still working.

The flags are named constants rather than written into the command so that tests
can read them, and **those tests are the only thing that would notice a removal**:
on every repository that does not exploit them, dropping them changes no output,
fails nothing, and produces exactly the same fingerprint. The one case where it
matters is the one case where noticing afterwards is too late.

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

## The content fingerprint

`content_fingerprint` walks the project and hashes what it finds. It needs no
Git, no repository and no history, and it is the kind every project gets that is
not at the root of its own working tree.

It is the same rule applied to a different source of paths. The walk is
`crate::scan` — the same ignore tables, the same comparison, the same limits — so
"generated and vendored churn is excluded" is not a claim made here and checked
somewhere else. `target/`, `node_modules/`, `vendor/`, `dist/`, `__pycache__`,
`.git/` and SURE's own `.sure/` are out for the reason the scan records beside
each of them, and `the_excluded_list_is_the_scans_and_not_a_second_copy_of_it`
pins the agreement rather than the list.

What is at each path is `read.rs`'s answer, shared with the Git kind: one
implementation of "what is at this path", not two that agree today.

**A link is in the manifest, and the scan did not put it there.** The scan skips
a link and records the skip as a *loss*, because a check does not read through
one and something could be hidden behind it. The content manifest resolves that
skip instead: the link is recorded by its target, exactly as the Git kind records
a tracked one. Leaving it out would be the dangerous direction — a link is a path
in the project and its target is the whole of what it is, so retargeting one is a
change, and a manifest that ignored links would report evidence about the old
target as current for the new one.

Every other loss **is** a loss: a directory that could not be listed, a level the
depth limit stopped, the point the entry budget ran out, a pipe met inside a
directory. Those become `IncompleteTree`, never a shorter digest.

### The digest, and the one property no test can hold it to

Same construction as the Git kind, with its own domain tag,
`sure.content-fingerprint.v1`, and the same four determinism decisions — the
paths are sorted before hashing, written with `/` on every platform, the branch
name is not a thing here, and every limit produces an error rather than a partial
manifest.

**The sort is held in place by its reason and not by a test, and saying so is the
point.** Removing it changes no observable behaviour in a single run: the digest
is a value, the sort exists to make that value independent of the order the walk
happened to produce, and one test run sees one filesystem's order and no other.
No two things a test can compare would differ. The Git kind's sort has the same
status. A mutation was applied that removed it and nothing failed; the verdict is
recorded in `target/tmp/mutate7.py` beside the property, and the reason is written
next to the code — which is the whole of what holds it.

The **domain tag** is in the same position, for a different reason: a mutation
that set it to the Git kind's was applied and every behavioural test still
passed, because the two digests cannot collide even with one tag — the Git kind
opens with `head` and this one with `manifest`. So the tag is belt and braces
over the field structure rather than the thing that keeps the kinds apart. It is
pinned by a constant assertion instead, which catches no behaviour and does
establish that changing it is a decision: the tag is part of the format of every
fingerprint already stored.

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

6. **A pipe, a socket or a device at a tracked path, on Windows.** A path that
   is none of a file, a directory or a link is described by its kind and never
   opened, because `File::open` on a FIFO with no writer **blocks until a writer
   appears** — and the working tree is written by whoever SURE is checking. A
   project could otherwise hang a check with no output, and a check that never
   returns cannot be told apart from one that is still working. The Unix test is
   the one that matters, since Unix is where the hang is possible; on Windows a
   pipe is not a filesystem entry, so Git cannot name one as a tracked path and
   the arm is unreachable there. Two facts about Git were measured while writing
   that test rather than assumed, and both are easy to get backwards: a tracked
   path whose working-tree entry is replaced by a pipe **is** reported, as an
   ordinary modified file, which is why the fixture commits before replacing it;
   and an *untracked* pipe is **not** reported at all, so nothing may be
   concluded from its absence from a fingerprint — it is absent from Git's
   answer first.

7. **The machine-config boundary is not tested through the product's entry
   point.** `the_two_switches_keep_a_machines_own_filter_out_of_the_answer`
   supplies a machine configuration and checks that two switches hide it and
   leave the repository's, but it drives `git config` directly. Driving
   `git_fingerprint` with an environment variable set would need
   `std::env::set_var`, which is **unsafe** since edition 2024, and this
   workspace forbids unsafe code. So the refusal path is covered end to end and
   the mechanism the boundary rests on is covered directly, but no single test
   holds both at once — they are joined by the unit test that pins the two
   variable *names*, which is the part a change could silently break.

8. **Two properties of the content manifest are held by reasoning and not by a
   test.** The sort before hashing, and the domain tag
   `sure.content-fingerprint.v1`. A mutation that removed the sort passed every
   test, because one run sees one filesystem's order and a digest is a value —
   there is nothing for a test to compare two of. A mutation that set the tag to
   the Git kind's also passed every behavioural test, because the two digests
   cannot collide even with one tag: the Git kind opens with `head` and the
   content kind with `manifest`. The first is held by the reason written beside
   it; the second is also pinned by a constant assertion, which catches no
   behaviour and does establish that changing the tag is a decision, since it is
   part of the format of every fingerprint already stored. Both verdicts are in
   `target/tmp/mutate7.py` rather than inferred from a green run — a property
   that looks covered and is not is the shape of failure this document exists to
   prevent. The Git kind's sort has the same status, which is why `mutate6.py`
   does not mutate it either.

Gaps 1, 3, 4 and 6 are stated in the module comment of
`crates/sure-core/tests/fingerprint_git.rs`, which is where a reader looks for
what a test file does *not* cover; that comment also names a fourth untested
case, `core.symlinks=false`, which is a branch of gap 1 rather than a separate
entry here. Gaps 1 and 6 are stated a second time, with the same cause and the
same platform, in the module comment of
`crates/sure-core/tests/fingerprint_content.rs` — the content kind's own link and
pipe tests are Unix-only for exactly those two reasons, so it is one missing
cover reached from two files rather than two gaps. Gap 2's tests are themselves
Unix-only, since producing a link on Windows is the thing gap 1 says this machine
cannot do — so on Windows the whole of the link behaviour, wider and blind alike,
is asserted by no test at all, and this document is the only place it is written
down. Gap 5 is a property of any design that reads files after asking Git about
them, and no test can pin it. Gap 6's Unix test is the one that can hang, so it
runs on a worker thread and fails by name after a timeout rather than sitting
there — a hang is a failure that reports nothing, and the point of the test is
that a project must not be able to cause one. Gap 7 is the one gap here that is a
property of the language rather than of the platform, and it is the only one that
will still be a gap if SURE is ever run on a machine unlike this one. Gap 8 is
the only one that is a property of the *design* rather than of a platform or a
language.

The same asymmetry runs through the "Enforced by" table below: the rows marked
(Unix) are the ones with no Windows test, and they are the rows about links,
about a file that cannot be read, and about pipes.

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
| A repository cannot make Git run a hook or a pager | this document | `the_settings_that_stop_a_repository_running_a_program_are_still_passed`, `the_config_arguments_are_the_ones_the_module_doc_explains` |
| A repository that names a program is refused | this document | `a_repository_that_names_a_program_to_run_is_refused_and_the_program_does_not_run` |
| …and without the program running first | this document | the same test, whose control runs the status and finds the marker, and whose second half finds it gone |
| …whether or not the machine has that program | this document | `a_repository_that_names_a_program_the_machine_does_not_have_is_still_refused` |
| A file naming a filter nothing defines is not a program | this document | `a_file_that_names_a_filter_nothing_defines_is_not_a_program` |
| A repository that stops naming one is ordinary again | this document | `a_repository_that_stops_naming_the_program_is_fingerprinted_like_any_other` |
| The refusal is over the setting's shape, not a list of names | this document | `every_shape_of_filter_setting_that_can_run_a_program_is_reported`, `a_setting_that_names_no_program_is_not_reported` |
| The machine's own filters are out of scope | gap 7 above | `the_two_switches_keep_a_machines_own_filter_out_of_the_answer` (drives Git directly), `the_variable_that_hides_the_machines_configuration_is_the_documented_one` |
| A pipe is described and not opened | this document | `a_tracked_path_replaced_by_a_pipe_is_a_change_and_not_a_hang` (Unix) |
| The digest is a correct SHA-256 | this document | `the_published_sha256_vectors_come_out_right` |
| Fields cannot be re-split across a boundary | this document | `the_framing_separates_a_split_in_a_different_place` |
| A Git digest is never a content digest | this document | `a_digest_of_one_kind_is_never_a_digest_of_another`, `the_two_kinds_cannot_produce_the_same_digest` |
| An absent half is not an empty one | this document | `an_absent_field_is_not_an_empty_one` |

The content kind, P2-T003:

| Statement | Where the meaning lives | Enforced by |
| --- | --- | --- |
| The same project read twice is the same manifest | this document | `the_same_project_read_twice_is_the_same_fingerprint`, `two_copies_of_one_project_are_one_fingerprint` |
| Excluded churn moves nothing, and a covered file moves it | this document | `churn_in_every_excluded_kind_of_directory_changes_nothing` and its control |
| The excluded list is the scan's, not a second copy | this document | `the_excluded_list_is_the_scans_and_not_a_second_copy_of_it` |
| A repository's own storage is out, so a commit moves nothing | this document | `a_repositorys_own_storage_is_out_so_a_commit_does_not_move_the_manifest` |
| A file's path is digested beside its contents | this document | `moving_a_file_without_changing_a_byte_of_it_is_a_change`, `swapping_two_names_is_a_change_even_though_no_bytes_moved` |
| A directory that was walked contributes what is inside it | this document | `a_directory_the_walk_did_not_descend_into_is_still_covered_by_what_it_holds`, `a_nested_checkout_within_a_project_is_covered_by_the_outer_manifest` |
| A missing project is refused, not read as empty | this document | `a_project_that_is_not_there_is_refused_rather_than_read_as_empty` |
| An empty project is a fingerprint and not a refusal | this document | `an_empty_project_is_fingerprinted_rather_than_refused` |
| A limit is an error, never a partial manifest | this document | `a_level_the_depth_limit_stopped_is_refused`, `a_walk_that_ran_out_of_budget_is_refused`, `more_files_than_the_limit_is_refused_rather_than_truncated`, `more_bytes_than_the_limit_is_refused_rather_than_truncated`, `the_byte_budget_is_spent_by_the_whole_manifest_and_not_by_each_file` |
| A relative root is refused | this document | `a_relative_root_is_refused` |
| A link is in the manifest, by its target, unread | this document | `a_link_is_recorded_by_where_it_points_and_never_read_through` (Unix) |
| A pipe is refused rather than opened | this document | `a_pipe_in_the_project_is_refused_rather_than_opened` (Unix) |
| A project at the root of its own repository is fingerprinted by Git | this document | `a_project_at_the_root_of_its_own_repository_is_fingerprinted_by_git` |
| A project in no repository is fingerprinted by content | this document | `a_project_in_no_repository_at_all_is_fingerprinted_by_content` — on a fixture that asserts it is outside one |
| A project inside somebody else's repository is fingerprinted by content | this document | `a_project_inside_somebody_elses_repository_is_fingerprinted_by_content`, `the_git_kind_moves_for_a_commit_the_project_is_not_part_of_and_the_content_kind_does_not` |
| …and that the Git kind would not do | this document | the same test, which asserts the Git kind *does* move for the same commit |
| A machine without Git is an error, not a fallback | this document | `a_machine_without_git_is_refused_rather_than_read_by_content` (a unit test — it needs a Git that is not installed) |
| The domain tag is the one stored fingerprints were written under | this document | `the_domain_tag_is_the_one_stored_fingerprints_were_written_under` — a constant assertion, and gap 8 says what that is and is not worth |

## Related decisions

| ADR | Topic |
| --- | --- |
| 0001 | Rust local core and crate boundaries — the fingerprint is in `sure-core` |
| 0002 | Local-first privacy — a fingerprint reads the project and sends nothing |
| 0005 | Evidence over claims — a fingerprint is what makes evidence expirable |
| 0007 | Windows primary development — case, long paths and links are primary cases |
| 0010 | Frozen domain semantics live in code — `GitState` and `FingerprintKind` are part of what a result means |
| 0012 | Diagnostics are records, not a logging framework — a fingerprint is a value on a result, not a log line |

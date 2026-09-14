# The component graph

The component graph is the second half of step 1 of
`docs/architecture/CHECK_PIPELINE.md`:

> Discover project/workspace, stacks, **components**, declared commands, config
> references and support level.

`docs/architecture/PROJECT_DISCOVERY.md` is the filesystem half of that step —
which files SURE will look at. `docs/architecture/ECOSYSTEM_DISCOVERY.md` is the
reading half — what each manifest says. This document is the structuring half:
turning what was read into **a list of the places in a project that are a thing
in their own right**. It is implemented by `sure_core::components`
(`crates/sure-core/src/components.rs`).

The problem it solves is that a monorepo read as one project is a project nobody
can check. "The tests pass" about a repository holding a web application, a
shared UI library and a database package is not a statement about any of the
three. A verdict has to be about a place, and a place has to be named before a
verdict can be about it.

## The one requirement

Everything here is arranged around a single sentence:

> **A component SURE did not read a manifest at does not have a stack.**

Not "has an unknown stack" — *does not have one*. The type says which of three
things is true about what a component is built with, and only one of them is
something a report may state as a fact:

| `Stack` | Means | May be reported as a fact? |
| --- | --- | --- |
| `Read` | Every ecosystem at this component had its manifest read. | yes, about *files* |
| `Partial` | Some were read and some were not. | no — an inference |
| `Unknown` | Nothing at this component was read. | no — an inference |

The distinction that matters most is the one between `Partial`/`Unknown` and a
plain wrong answer. A member directory whose `package.json` SURE did not open is
not a component with an unknown framework. It is **a directory a workspace
declaration named and SURE knows nothing else about** — and a reader told the
first thing goes looking for a framework nobody ever claimed. That is the shape a
false green takes at this stage: not a wrong verdict, but a true-sounding noun
attached to no evidence.

## Why the readings are per ecosystem

A `Component` holds `manifests: Vec<ComponentManifest>`, one entry per ecosystem
at that component, rather than a single merged verdict.

A merge is where the bug would live. Node and Rust can name the same directory;
one of them read a manifest there and the other did not. A single field has to
pick one, and either choice silently discards a fact — in one direction it
promotes a guess to a finding, in the other it hides something SURE actually
read. Keeping both means **there is no merge to get wrong**, and `Stack` is a
derivation over the list rather than a stored summary that could drift from it.

`ManifestReading` has five arms because discovery established five different
things, and none of them is another:

| `ManifestReading` | What discovery saw |
| --- | --- |
| `Read` | The manifest was parsed. |
| `Unread(reason)` | A read was attempted, and `UnreadReason` records why it failed. |
| `NotOpened` | The manifest's name is there and nothing was attempted — a member past the manifest budget. |
| `NotAFile { kind }` | Something is at the name and it is not a file SURE reads. |
| `NoManifest` | This ecosystem is here and has no manifest of its own at this component. |

`NotOpened` and `NoManifest` are the pair most easily collapsed by accident, and
collapsing them is the error: the first says *there is a `package.json` SURE did
not open*, the second says *there is nothing there*. A report that confused them
would tell a reader either that a file is missing when it is not, or that a file
was read when it was not.

`Unread` carries an `UnreadReason` rather than a `String` because `UnreadReason`
already draws the line this document needs: its `plain_description` is all
constants, and the variable parts — an operating system message, a parser
position — are separated into `detail` and are *reported* rather than composed
into a sentence. That is the division `docs/architecture/EVIDENCE_MODEL.md`
makes between what SURE says and what something else said.

## The three member answers

`ComponentGraph::is_multi_component` answers "is this project more than its
root?" — and it is only allowed to answer `false` as *"this is one component"*
when **every** ecosystem agreed. `resolution` carries one `Members` entry per
ecosystem, always, whether or not that ecosystem was found:

| `Members` | Means | `is_known_single()` |
| --- | --- | --- |
| `NoProject` | SURE looked for this kind of project and found none. | `true` |
| `NoWorkspace` | SURE read the declaration and it names no members. | `true` |
| `Resolved { truncated }` | A member list was read, complete or not. | n/a |
| `NotRead { because }` | SURE did not read the declaration that would say. | `false` |

`NotRead` exists for one reason: **a graph that is one component wide because an
ecosystem never looked must not read as a project with one component.** Those are
different claims and the second is much stronger than the data supports.

Today `NotRead` is returned for **every** Python project, because Python resolves
members through `[tool.uv.workspace]` and `[tool.pdm.workspace]` tables that
`crates/sure-core/src/discover/python.rs` does not read. SURE cannot tell a
single-package Python project from a hundred-package one, so it says so rather
than guessing. This is known coverage gap 8 in
`docs/architecture/ECOSYSTEM_DISCOVERY.md`, and the component graph is where it
would otherwise become a false claim.

`Resolved { truncated: true }` is the other caveat that cannot be dropped: it
means there were more members than `DiscoverOptions::max_workspace_members` and
this graph is a prefix. `ComponentGraph::plain_description` names both caveats in
the sentence it produces, so a caller that never looks at `resolution` still
cannot report the count without them.

## Containment

`contains` holds one edge per non-root component, to its **nearest** enclosing
component. A member inside a member is one edge and not two — `packages/app/deep`
is contained by `packages/app`, and only transitively by the root.

Containment is decided by `Path::starts_with`, which compares **path
components** and not text. `packages/application` does not start with
`packages/app`, and a check written as a string prefix would say it did.

What `contains` is *not* is a dependency graph. A `package.json` saying
`"@app/ui": "workspace:*"` is a request to a resolver SURE has not run, and
drawing it as an edge would assert a resolution that never happened. `contains`
is a fact about the paths in the walk; `declared_by` is a fact about files SURE
read, and it carries its `Source`.

## What the graph does not do

**It does not read anything.** It is a view over a `Discovery` and opens no file.
If it opened one it could disagree with the discovery it came from about what was
in the project, and a graph that disagreed with its own evidence is the failure
this product exists to prevent. This is enforced against the source, not asserted
in prose — see the table below.

**It does not resolve dependencies between components.** See above.

**It does not decide what kind of thing a component is.** Application, library
and service are not distinguishable from a manifest SURE has read. A guess about
which one a directory is would be exactly the inference this module refuses to
promote, so `Component` has no `kind` field at all.

**It does not apply `exclude`.** See gap 1.

**It does not order components by importance, size or anything but path.**
`components` is sorted by `Path`'s own order — component by component, so the
root (the empty path) is first and `packages/alpha` precedes `packages/zeta`
whatever either is made of. Two runs of one project print the same list, and the
order does not depend on which ecosystem discovery happened to read first.

## Known coverage gaps

Each is recorded so it is not forgotten. None is resolved by a task yet.

1. **A Rust member that `exclude` names is still a component here.** Discovery
   reports those paths in `Workspaces::excluded_members` and deliberately does
   not subtract them — whether `exclude` removes a member is Cargo's rule, and
   SURE has not read it. The graph inherits that decision, which is right, and
   the cost is that the graph offers no way to see which components are the
   excluded ones. A caller that needs it must reach `excluded_members` through
   `Discovery` itself. This is gap 16 of
   `docs/architecture/ECOSYSTEM_DISCOVERY.md` seen from the other end.

2. **A directory no workspace pattern names is not a component, even if it has
   its own manifest.** `packages/thing/package.json` in a project whose root
   `package.json` declares no `workspaces` produces one component, not two.
   Discovery reads the root manifest and the members a declaration named, and
   nothing tells it that an unlisted directory is a separate thing — the scan
   knows the file exists and has no reason to think it matters. Guessing from the
   directory name would be inference presented as structure.

3. **A Python monorepo is one component with a caveat.** Gap 8 of
   `docs/architecture/ECOSYSTEM_DISCOVERY.md`. The graph reports `NotRead` so the
   caveat cannot be lost, but the members are not enumerated: a caller that wants
   per-package verdicts for a uv workspace cannot have them yet.

4. **A truncated member list is a prefix, and which members are missing is not
   known.** `truncated: true` is reported and the missing names were never
   resolved, so there is nothing more to say. This is a property of the limit
   rather than a defect, and it is here because a prefix presented as a whole
   workspace is the false claim the flag exists to prevent.

## Enforced by

| Statement | Where the meaning lives | Enforced by |
| --- | --- | --- |
| A component SURE did not read has no stack | this document, `components.rs` | `a_member_past_the_manifest_budget_is_not_opened_and_not_absent`, `a_member_whose_manifest_is_not_a_file_is_an_inference_and_not_a_stack`, `a_member_sure_did_not_read_comes_back_as_an_inference_from_the_public_api` |
| An unread manifest is not an absent one | this document | `a_member_past_the_manifest_budget_is_not_opened_and_not_absent`, `a_root_manifest_that_could_not_be_parsed_is_unread_rather_than_absent` |
| A member with nothing at the name is told apart from one that was not read | this document | `a_directory_named_as_a_member_with_no_manifest_has_no_manifest_and_no_stack` |
| Readings are kept per ecosystem and never merged | this document | `a_directory_two_ecosystems_name_is_one_component_keeping_both_readings` |
| One directory named twice is one component | this document | `a_directory_two_ecosystems_name_is_one_component_keeping_both_readings` |
| An ecosystem that did not look does not answer the single-component question | this document | `the_three_member_answers_are_three_different_things` |
| Python's unread member list is a caveat and not "none" | this document, gap 3 | `python_reports_its_members_as_not_read_rather_than_as_none`, `a_python_project_does_not_read_as_one_component_it_did_not_verify` |
| The caveat reaches the graph's own sentence | this document | `python_reports_its_members_as_not_read_rather_than_as_none` |
| The root is always a component | this document | `a_project_with_no_workspace_is_one_component_and_says_so` |
| A root SURE read nothing at has no stack | this document | `the_root_of_a_project_sure_read_nothing_at_has_no_stack` |
| Every named member is a component the root contains | this document | `every_named_member_is_a_component_and_the_root_contains_it`, `a_monorepo_reads_as_several_components_from_the_public_api` |
| Containment is to the nearest enclosing component, one edge each | this document | `a_member_inside_a_member_is_contained_by_its_nearest_parent` |
| Containment is by path component, not by text | this document | `a_containment_is_by_path_component_and_not_by_the_text_of_a_path` |
| A dependency is not a component | this document | `a_dependency_naming_a_workspace_package_does_not_become_a_component` |
| The graph opens no file and starts no process | this document | `the_component_graph_opens_no_file_and_starts_no_process` |
| The graph is a view, not a re-read | this document | `the_graph_reads_nothing_of_its_own` |
| The component order does not depend on which ecosystem was read first | this document | `the_components_are_in_one_order_whichever_ecosystem_named_them` |
| No project text reaches a sentence | `docs/architecture/EVIDENCE_MODEL.md` | `every_component_carries_the_files_that_named_it_and_no_project_text` |

## Related decisions

| ADR | Topic |
| --- | --- |
| 0001 | Rust local core and crate boundaries — the graph is a module of `sure-core`, beside `discover` |
| 0002 | Local-first privacy — the graph holds paths and constants, and nothing it carries leaves the machine |
| 0005 | Evidence hierarchy — an inference and a read manifest are different kinds of evidence, so they are different variants |
| 0007 | Windows primary development — containment is by path component, which is the comparison that behaves the same on a case-insensitive volume |
| 0010 | Frozen domain semantics live in code — `Stack` and `Members` are part of what a report means |

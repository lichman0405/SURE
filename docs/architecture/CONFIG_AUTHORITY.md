# Configuration authority

The checked project may be controlled by the same AI whose work SURE is evaluating. Therefore project-controlled configuration is not trusted to grant SURE additional authority.

## Authority order

Highest authority:
1. explicit interactive/user approval for the current action;
2. user-level SURE configuration stored outside the project;
3. organization policy (future team edition, where applicable);
4. project `sure.yaml` suggestions;
5. inferred defaults.

A lower-authority source cannot weaken a higher-authority safety/privacy restriction.

## Project config may

- select/disable noncritical checks;
- describe project components;
- provide explicit project goal/spec paths;
- suggest start/test commands;
- configure report preferences.

## Project config may not silently

- enable host execution if the user did not allow it;
- enable dependency installation/network access;
- disable user protection rules;
- enable full transcript recording;
- provide/override secret credentials as an authority bypass;
- delete authoritative history.

If project config requests a privileged behavior, SURE treats it as a request requiring higher-authority approval.

## What is implemented, and what is not

Ranks 2 and 4 exist and are resolved against each other by
`crates/sure-core/src/config/authority.rs`. Rank 1 is a decision rather than a
file and is recorded as `ConsentGrantor::InteractiveUser` when it exists; the
prompt it comes from is P3-T005. **Rank 3 does not exist in this release**, and
neither `Layer` nor `ConsentGrantor` offers a way to name it — a source a caller
can name but never obtain is how a documented feature becomes a believed one.

Read plainly: this layer decides what the two files, together, are allowed to
mean. **Nothing routes through it yet.** No command builds an `Authority` today;
wiring it in front of the check pipeline is P13-T009. Until then the resolution
is built and tested on its own, and a report that claims a project's request was
refused would be describing behaviour that has not run.

## The two answers

A merge — folding the project's settings into the user's, keeping whichever is
safer per field — was rejected (ADR 0011). It reinterprets intent: a user who did
not mention a setting has not expressed a preference to be maximised, and the
merged result cannot be reported back in terms of the files they wrote.

Two answers are well defined, and they are the two the layer gives.

### Requests: every ask is on the record

`Authority::privileges()` returns one entry per behaviour any layer asked for. A
`Privilege` names the request, every layer that asked for it (most trusted
first), and who was able to grant it. Four outcomes, all of them visible:

| Who asked | Who granted | What it means |
| --- | --- | --- |
| the user's file | the user's file | allowed |
| the user's file and the project's | the user's file | allowed, and the project agreed |
| the project's file only | nobody | **refused** — a recorded escalation |
| nobody | — | not in the list at all |

The refusal is a value rather than an omission. A file that asked for network
access and did not get it leaves a `Privilege` behind, so a report can say what
was asked for. Dropping it would make "the project asked and was refused" and
"the project asked for nothing" the same list, which is the shape of report this
product exists to replace.

`Authority::permissions()` is the permission set those grants add up to. It is
not the answer to whether an action may run: that is
`sure_domain::execution::decide`, which also needs a mode and treats a command it
cannot classify as needing its own consent in every mode.

### Restrictions: the stricter of the two wins

Protection and privacy mode resolve to the stricter value either layer set, and
`Resolved::by` names the most trusted layer that asked for it. A project may ask
for *more* protection and is named as the reason; it may not ask for less.
`fully_local` beats `local_first` whichever layer wrote it, because sending less
out is never the escalation.

A layer that asked for nothing is `by: None`, which means "nothing beyond the
default" and not "SURE did not work it out". A report that could not tell those
apart would be unable to say whether it had looked.

## What this deliberately does not do

**There is no merged `Config`.** There is no `Authority::effective()`.

**Scope reductions are not overridden, only reported.** A project may turn a
check off; `Config::scope_reductions` says so. The user's file cannot be said to
have overridden it, because a project that sets a check to the value SURE uses by
default is indistinguishable from one that never mentioned it — the claim would
be one SURE cannot support.

**Nothing here is a consent prompt.** An interactive approval is per action and
names the exact command; a configuration file grants a class. The two are
different objects and are not merged into one "allowed" flag.

## Enforced by

| Where | Holds |
| --- | --- |
| `crates/sure-core/src/config/authority.rs` | the layers, the resolution, and its unit tests |
| `crates/sure-core/src/config/mod.rs` | `Config::load_file`, and `requested_privileges` / `scope_reductions` |
| `crates/sure-domain/src/execution.rs` | the frozen vocabulary: `Permission`, `ExecutionPermissions`, `ConsentGrantor`, `decide` |

`crates/sure-core/tests/config_loading.rs` covers the filesystem path a project
file is read from. The user's file is read by the same code, at the path
`Paths::user_config_file()` names.

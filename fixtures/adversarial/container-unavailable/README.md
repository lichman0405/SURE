# container-unavailable

## What is in this directory

A project with one check in it, and a computer with no container runtime on it —
which, in this build, is the same answer as a computer that has one: nothing
runs the check, here or in a container.

| File | What is in it |
| --- | --- |
| `src/run-check.js` | the check: it adds up two monthly totals and fails loudly if a row is not a number |

There is no `package.json`, no `pyproject.toml` and no `Cargo.toml`, and that is
deliberate. The absence this fixture is about is a fact about the *computer*, not
about the project, so there is no language half to run and nothing for a
runnability sweep to execute. Nothing here is installed and nothing is fetched.

## What SURE does with it

`Availability::in_path` is the question *is there a container runtime on this
search path?* and its answer is a value with two shapes and no error:

```
Found { runtime, program }   something was found, here it is
Absent                       nothing was found
```

There is no third shape, and in particular there is no *failed* shape. That is
the whole of the honesty this fixture is about: a missing runtime is not an
exception to catch, not a defect to report and not a silence. Ask a path with
nothing on it and SURE says what happened, what it does not change, and what it
looked for, in one sentence:

```
No container runtime was found, and that changes nothing: this build runs no
check in a container, and no check runs on this computer unless your own
settings allow it. SURE looked for docker or podman on PATH.
```

The test compares that sentence whole rather than searching it for a word, and
it has to do three things at once — say what was not found, say that **no check
runs in a container and none runs on this computer unless the user's own
settings allow it**, and name what was looked for — or a person reading it
cannot act on it.

Ask a path that holds a file named `docker` and SURE answers the other way, in a
sentence built the same way:

```
docker was found at <the file that was found>, and that changes nothing: this
build runs no check in a container, and no check runs on this computer unless
your own settings allow it.
```

**`P18-T007` moved this sentence and the move is deliberate rather than
cosmetic.** It read *"this build runs no check, in a container or on this
computer, and each check is recorded as unknown rather than passed"*, which was
true of a build whose pipeline admitted every check and ran none. That is no
longer this build: a run the user's own settings file grants now reaches the
runner through `sure_core::pipeline`, so the old middle clause names a state
that stopped existing and the new one states the rule that replaced it — no
check in a container, ever, in this build; on this computer only on a grant.
The declaration was re-made at the same time as the sentence, and the
`module_call_sites` count beside it was held to the scanner rather than
adjusted.

**The two answers are one sentence with one clause moved, and since `P16-T012`
that is asserted rather than intended.**
`crates/sure-core/tests/container_isolation_claim.rs` holds both arms to the same
clause word for word, and to the rules that read a claim of running whatever
wording it uses. The arm with a runtime is the one that was wrong: it read
*"Checks can run in a container: docker was found at …"*, and **no rule in the
module read it** — `sure_core::container::claims_local_execution` is about the
fallback claim and needs a place on this computer, and *"in a container"* is not
one. So a machine with a runtime on it was told its check could run, in a build
whose pipeline ran nothing. `sure_core::container::claims_container_execution` is the
twin rule written for it; the two share one implementation and differ in the list
of places they read, which is why the sentence could be wrong on one side of the
question without the other side noticing.

**The absence sentence used to read *"No container runtime was found, so checks
run on this computer instead."*, and this README used to explain the requirement
in the same false way.** The guard was green because it asked the sentence for a phrase
rather than for a true claim, and the phrase it asked for — a fallback onto this
computer — was false of the build it was written in, where no check ran
anywhere: `sure_core::enforce` says no plan was driven down the road to
`Enforcement::admitted()`, and `sure_core::support`'s `CEILING` is `InspectOnly`
for the same reason. It is
written down here so that nobody restores it. An absent runtime changes *inside
what* a command would run and not *whether* anything runs, so a sentence that
names a fallback is wrong in any wording, not only in the old one —
`sure_core::container::claims_local_execution` is the rule that reads the claim
rather than the phrase, and it is what fails if one comes back. **`P18-T007`
sharpened that argument rather than retiring it**: what decides whether a check
runs on this computer is now a grant in the user's own settings file, and a
missing runtime still decides nothing, so a sentence that named a fallback would
be wrong for a second reason as well as the first.

## The machine is never the measurement

**No assertion in this fixture depends on whether the machine running it has
Docker.** A fixture that recorded *no runtime found* on the laptop it was written
on would be a test of the laptop: the same program would answer differently on a
build machine, and a green run would say nothing about SURE. So the search path
is a parameter rather than something read out of the environment, and both
answers are produced on every machine:

| Search path | Answer |
| --- | --- |
| nothing at all | `Absent` |
| a directory this test owns, holding a file named `docker` | `Found { Docker, <that file> }` |

Same function, same machine, same process, one argument apart. `Found` is
compared by equality on the whole value including the path of the program that
was found, so an answer that guessed a runtime instead of reading the directory
would fail. A third directory holds only `podman`, which is what makes the
*reported name* a measurement rather than a preference: the name that comes back
is the name that was written.

The one call that does read this computer's own `PATH` is
`Availability::on_this_machine()`, and the test asserts its **shape** only — that
it is one of the two variants, that any runtime it reports is one the search
would have looked for, that the sentence it produces is non-empty and carries no
overclaim, and, when it reports having found nothing, that the sentence is this
fixture's sentence word for word. Nothing else about it is asserted, because
nothing else about it is a fact about SURE — in particular the words of the
sentence *the probe* produces on a machine that has a runtime are not asserted
anywhere, since on such a machine that would be a test of the machine. The
sentence a run of the control produces is asserted, and that is a different
thing: it is built from a search path the test wrote, and the one part of it the
test cannot fix — the path of the program that was found — is substituted into
the declaration before the two are compared.

## What the mode is allowed to say about itself

A container is not a sandbox, and the sentence that describes the mode says so
in as many words:

> This is limited isolation: it narrows what a check can touch, and it is not a
> sandbox.

That sentence is quoted whole here and forbidden from using `isolated container`
or `isolated environment` as a claim, by the module's own `overclaims` rule
rather than by a search for a word. It is the one sentence a person reads
*before* agreeing to run a stranger's code, so it is the one place where saying
too much is expensive.

## The limit, recorded rather than repaired

**Container execution is not reachable from the pipeline or the command line.**
`crates/sure-core/src/container.rs` is declared in `crates/sure-core/src/lib.rs`,
the diagnostic asks it what is on the search path it was handed, and nothing that
ships can turn a check into a container command.

Those are two claims, and the fixture declares one number for each rather than
one number for both:

| Declared | What it counts |
| --- | --- |
| `module_call_sites: 18` | lines of shipped code under `crates/` that name `container::` or `Availability::` |
| `unwired_call_sites: 0` | among those lines, the ones naming `ContainerPlan` or `PlanError` — the type a container command is built from, and the error building one can fail with |

The first is not zero, and it is not meant to be: since `P15-T001` the diagnostic
reports what is on the search path a caller handed it, and the guards that read
the sentences it prints name the module too. The second is the limit, and it is
zero: no line of shipped code can build a container command, so no check can be
told to run in one. **A number that moved is not a limit that changed**, which is
why both are declared and both are compared. `P16-T010` moved the first from 11
to 16 with five assertions, and `P16-T012` moved it from 16 to 18 with two more —
each time because a guard learned to name the module, and never because anything
became reachable. The second number has never moved.

The test measures that rather than describing it: it walks the tree with the
product's own scanner, asserts the walk was complete and non-trivial, strips
line comments, and compares both lists against the numbers this fixture declares.
Deleting the diagnostic's answer would fail on `module_call_sites`; wiring a
container command in would fail on `unwired_call_sites`, and on the assertion
above it that prints the line that did it. This entry is what will have to
change, deliberately, when a later phase wires the mode in.

Consequences worth stating plainly, because they are what a reader would
otherwise assume away:

- SURE produces **no finding** for this fixture, no candidate and no
  not-checked row. The project's one check is not planned at all: with a runtime
  on `PATH` and without one, this build plans exactly the same checks, which is
  nothing. A runtime appearing on this computer would not change a single answer
  SURE gives today.
- The `expected_severity` and `release_blocking` at the top of `scenario.json`
  are therefore about the level the product actually reaches, not a row copied
  from the release contract. There is no case for this fixture in
  `evaluation/acceptance-manifest.json`: the manifest's cases are about defects
  in a project, and this is about a machine.
- The nearest case by name is `dynamic-not-authorized`, and it is a different
  defect: there a check was *refused* by a rule about consent and the refusal is
  the thing that must stay visible; here nothing is refused by the absence at
  all.

## How to see it for yourself

```
cargo test -p sure-core --test adversarial_fixture_detection -- a_missing_container_runtime
```

The test reads `scenario.json`, calls `Availability::in_path` with an empty path
and with directories it creates under the repository's own ignored `target/tmp`,
asserts both whole answers against the fixture's declaration, quotes the
sentences, walks the sources for the limit above, and removes the scratch
directories before it ends. It starts no container runtime and installs nothing.

## Watch out

This fixture has no control that changes the *project*, because the project is
not what the case is about. The control moves the search path and nothing else —
and if a later build decides that a missing runtime is a failure, or that it is
the project's fault, or that it is worth saying `sandbox` about, the sentences in
this directory are the ones that will disagree.

# container-unavailable

## What is in this directory

A project with one check in it, and a computer with nowhere to run that check
apart from the computer itself.

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
nothing on it and SURE says what happened and what happens instead, in one
sentence:

```
No container runtime was found, so checks run on this computer instead.
SURE looked for docker or podman on PATH.
```

The test compares that sentence whole rather than searching it for a word, and
it has to do three things at once — say what was not found, say what happens
instead, and name what was looked for — or a person reading it cannot act on it.

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
it is one of the two variants, that the sentence it produces is non-empty and
carries no overclaim, and, when it reports having found nothing, that the
sentence is this fixture's sentence word for word. Nothing else about it is
asserted, because nothing else about it is a fact about SURE.

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
`crates/sure-core/src/container.rs` is declared in `crates/sure-core/src/lib.rs`
and nothing else in the shipped tree calls it: across the non-test Rust files
under `crates/`, no line of code outside the module itself names `container::` or
`Availability::` — only comments do.

The test measures that rather than describing it: it walks the tree with the
product's own scanner, asserts the walk was complete and non-trivial, strips
line comments, and compares the list of call sites against the number this
fixture declares (`unwired_call_sites: 0`). Wired in and recorded as unwired
would fail here; deleted and recorded as wired would fail here too. This entry is
what will have to change, deliberately, when a later phase wires the mode in.

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

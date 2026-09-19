# dynamic-not-authorized

## What is in this directory

A very small Node project with one declared script, and one authorisation it
does not have.

| File | What is in it |
| --- | --- |
| `package.json` | the project, with one script: `"test": "node src/totals.test.js"` |
| `src/totals.js` | the code the script checks |
| `src/totals.test.js` | a hand-rolled check, using only Node's own `assert` |
| `sure.yaml` | the project **asking** for its own checks to be run |

Nothing here is broken, nothing is a placeholder, and nothing needs installing:
the check is four lines and a builtin module.

## What SURE does with it

The declared `test` script becomes one planned check — `run the tests`, severity
`must_fix`, critical — and SURE is not allowed to run it, because the person at
the keyboard has not said it may. So the check comes back as a check that did
not happen:

```
status                            skipped
reason (the frozen sentence)      Checking this would have meant running your project's code,
                                  and you have not allowed that.
```

That sentence is fixed in `crates/sure-domain/src/status.rs` for exactly this
reason. The test compares the whole of it, and the whole of the rendered report,
so a build that changed the words or stopped printing them would fail.

## It must stay visible

This is the whole fixture. The tempting failure is not a wrong word — it is a
check that disappears:

```
Not enough could be checked to say whether this is ready.
This project is not ready to hand off.
...
No open findings.
1 check(s) could not run or were skipped. 1 of them are critical. (run the tests)
```

Drop that last line and the report reads as a project with nothing outstanding.
It is not. Nothing was checked at all: `coverage_checked` is zero, and the one
thing SURE would have had to run to say anything is the thing it was not allowed
to run.

So the check is asserted in four separate places, because a field on a struct is
not a report a person reads: in the verdict's list of checks that did not run, in
the aggregate's list of critical checks that were not checked, in the coverage
summary, and in the last line of the summary above.

It must **not** be reported as a defect of the project either. The script is
declared, the file it names exists, and the reason it did not run is a permission
the user has not given. A finding here would blame a project for something a
person decided.

## Why the project's own `sure.yaml` is here

`sure.yaml` says:

```yaml
execution:
  mode: host_confirmed
```

It gets nothing. A project file may ask, and only the user's own configuration
file — outside the project — may grant: that is the rule in
`docs/architecture/CONFIG_AUTHORITY.md`, and the fixture ships an asking project
so that a build which read execution authority out of the repository would be a
red test rather than an undetected escalation. The test asserts the request was
recorded, that it was refused, and that the run still planned under
`inspect_only`.

## The control, and why the fixture is worthless without it

A reporter that answered *"you have not allowed that"* to **everything** would
satisfy every sentence above and would be measuring nothing at all. So the same
project is run again with one thing moved, and it is not in the project:

> the user's own configuration file appears, saying `execution: mode:
> host_confirmed`

The file is written under a configuration root this test owns — `Paths::from_roots`
puts SURE's configuration root under `target/tmp` for the length of the test, and
`Authority::load` reads it the way `sure check` does. The machine's real
configuration directory is never read and never written. The same directory is
read by the same call, so nothing else can have moved.

With the grant, the answer must change:

- the check is no longer refused: **no result carries the not-authorized reason
  at all**;
- the plan's decision for it is `allowed` instead of `denied`;
- what it becomes is `unknown` with *Nothing was reported for this check, so SURE
  has no basis for a verdict on it.* — **not** a pass.

That last point is the point. Nothing in this build carries a planned check out,
so consent changes what SURE is *allowed* to do and not what it has *done*: the
coverage still shows nothing checked, the aggregate is still
`not_enough_checked`, and the rendered summary is the same six lines, word for
word, on both sides. If a later build learns to run the check, this declaration
is what has to be re-made — and it should be re-made deliberately rather than
discovered.

## How to see it for yourself

```
cargo test -p sure-core --test adversarial_fixture_detection -- an_unauthorised_dynamic_check
```

That test reads `scenario.json`, drives SURE's real pipeline over this directory
with execution unauthorised, asserts the status, the reason, the frozen sentence,
the four places the check stays visible and the whole rendered summary, then
writes the user's grant into a configuration root it owns and drives the same
pipeline again and asserts the opposite. Nothing is run inside the project and
nothing is written outside `target/tmp`.

The fixture has a case in `evaluation/acceptance-manifest.json` —
`dynamic-not-authorized`, not release-blocking, expected severity `note`,
expectation *unexecuted dynamic checks stay visible as not checked/skipped* — and
the two fields at the top of `scenario.json` are copied from that row rather than
chosen here. SURE produces no finding for this case at all, which is under that
ceiling rather than at it.

## Watch out

The `must_fix` on the check is the weight of the check, not a verdict about the
project: the report's material and style-noise buckets are both empty, and the
test asserts that rather than leaving it to be assumed. A reader who takes
*1 of them are critical* for a failing test has misread a check that never ran
for one that ran and failed.

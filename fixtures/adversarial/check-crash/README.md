# check-crash

## What is in this directory

A schedule and a set of runs, and no project at all.

| File | What is in it |
| --- | --- |
| `scenario.json` | two check declarations, two execution modes, and six runs |

There is no `package.json`, no `pyproject.toml`, no `Cargo.toml`, no `src/` and
no `entry_points`, and that is deliberate. The case is **a critical check that
produced no result**, not a project with code in it, so there is nothing here to
discover and nothing to run. The schedule is built from the declaration by the
product's own `PlanBuilder` (`crates/sure-core/src/schedule.rs`), and each run
hands that schedule and a set of results to `aggregate_run`
(`crates/sure-core/src/aggregation.rs`) — the one function in the tree that
decides what a check with no result is.

## The trap this fixture exists to catch

The acceptance is *"critical error/skipped/unknown cannot aggregate green"*, and
it is a **negative** claim. Three different broken things satisfy it:

1. **A product that never produced a result at all.** `aggregate_run` walks the
   *schedule* rather than the results, so a check nobody reported on becomes a
   row only because that walk puts it there. Drop the check from the schedule and
   the criterion still holds — vacuously, with nothing recorded.
2. **A product that answers `NotEnoughChecked` for everything**, including a run
   in which every critical check passed.
3. **A test that asserts only `!severity.is_green()`.** That assertion is true of
   `NotEnoughChecked`, `NotReady`, `NeedsAttention` **and** of an empty plan,
   because `aggregate(&[])` is `NotEnoughChecked` too. It cannot tell *the
   critical error was seen and reported* from *the run was non-green for some
   other reason*.

So nothing here is asserted as "not green". Every run declares the **exact**
severity it must reach, every row declares the state and the sentence it must
carry, and one run must reach **`Green`**.

## The six runs, and what each one is for

| Run | What the plan said | What came back | Severity |
| --- | --- | --- | --- |
| `checker_failure_unknown` | one check, allowed | nothing | `not_enough_checked` |
| `checker_failure_skipped` | one check, refused by the mode | nothing | `not_enough_checked` |
| `checker_error` | one check, allowed | the checker failed | `not_enough_checked` |
| `scope_limit_alone` | one out-of-scope check, allowed | an out-of-scope skip | `not_enough_checked` |
| `scope_limit_beside_a_pass` | the same skip plus a passing check | both | `needs_attention` |
| `control` | one check, allowed | **a pass** | **`green`** |

**`control` is the acceptance.** Everything above it is satisfied by a product
that answers `not_enough_checked` to everything, and the control is the only run
that says otherwise. `scope_limit_beside_a_pass` says it from the other side, on
an answer no other run requires.

## Which route produced each of `Error`, `Skipped` and `Unknown`

This is stated plainly because the three are not alike **in this directory's
runs**: two of them are values the test reads out of a run the product performed,
and one is a value the test builds, because this directory ships no project and
so there is no run here that could produce it. That last part is a limit of the
fixture and **not** a claim that the status is unreachable in the product — it
is, and the `Error` bullet below names where.

- **`Unknown` — a product-computed value, through the product's real route.**
  `aggregate_run`'s `(None, None)` arm builds it for a scheduled check the plan
  allowed and for which nothing came back, and the sentence it carries is
  `NOTHING_CAME_BACK`, a constant in `crates/sure-core/src/aggregation.rs`. That
  constant is public so the test can hold the produced sentence against **it**
  rather than against the copy in this directory: a second copy would keep
  agreeing with itself after the product's sentence was rewritten.
- **`Skipped` — a product-computed value, through the product's real route.**
  `ScheduledCheck::not_run` builds it for a check the execution mode refuses, and
  the reason is the vocabulary's `NotCheckedReason::ExecutionNotAuthorized`. The
  test holds the sentence against
  `ExecutionNotAuthorized.plain_explanation()` — the product's own method — for
  the same reason as above. This is the exact route
  `crates/sure-core/src/pipeline.rs` takes for every check the default mode
  refuses.
- **`Error` — a route that real runs take, and a result this directory has to
  build.** These are two different statements and the distinction is the point.

  **The route is shipping code, and a reader can go and look.** `CheckStatus::Error`
  → `CriticalState::CheckerError` → `critical_errored` → `not_enough_checked` is
  reachable from a real run, by at least three paths:

  - `crates/sure-core/src/probe.rs` — `ProbeOutcome::Unreachable` maps to
    `CheckStatus::Error` (`status()`, line 880), and `ProbeOutcome::verdict`
    (line 791) passes the caller's own `severity` and `critical` straight through
    to `CheckResult::errored` (line 823). A **critical** probe that could not be
    made is therefore a critical errored result, and `critical_errored` is
    non-empty for it.
  - `crates/sure-core/src/pipeline.rs` — `check_claims` (line 994) returns a
    `CheckResult::errored` (line 1051) when the claim-checking stage cannot read
    what it recorded, and that result is `extend`ed into `results` (line 763) and
    handed to `aggregate_run` (line 767) on the main run path. That one is
    `Severity::Note` and `critical: false`, so it lands in `counts.error` and
    `not_checked()` rather than in `critical_errored` — the coverage loop skips
    non-critical results (`crates/sure-domain/src/status.rs`, line 631). The
    status is still produced by a real run.
  - `crates/sure-core/src/browser.rs` (line 708) and
    `crates/sure-core/src/runtime_start.rs` (line 434) map a status they do not
    expect to `errored` rather than let it through as a pass, again passing the
    check's own `severity` and `critical` through.

  **What is a limit is narrower than "the product cannot produce one".** This
  directory ships no project and no assertion here runs the pipeline over one
  (`scenario.json`, `checker_failure.why_not_the_pipeline`), so the test cannot
  provoke any of those routes: it builds the declared result with
  `CheckResult::errored` and hands it to `aggregate_run`. **That reason used to
  be a different one** — "this build has no runner for a planned check" — and
  `P18-T007` made it false by wiring the pipeline to the runner; the fixture's
  own reach is what it always was, and every value this file builds is still
  built for the same reason. And the **detail
  sentence** is a copy agreeing with itself — the product has no sentence for a
  checker error, because the text is whatever the checker said, so the test
  supplies this fixture's string and reads it back. Those two are the limits, and
  they are limits of *this fixture's reach*, not of the product.

  What is **measured** is everything downstream of the status:
  `CheckStatus::Error` → `CriticalState::CheckerError` → `critical_errored` →
  `not_enough_checked`, and an evidence class that `CheckResult::errored` does not
  take as a parameter and so cannot be written wrongly. The same is true, in a
  smaller way, of the `scope_limit` runs: a scope-limited skip is a hand-built
  result too, because `ScheduledCheck::not_run` only ever produces
  `ExecutionNotAuthorized`.

The control's `pass` is a hand-built result for the same reason, from the other
end: **no run in this fixture reaches the pipeline**, so the check that has to
come back green is declared here rather than produced. That was once the same
statement as "this build plans every check and runs none of them" — and the day
the second half stopped being true is `P18-T007`, the day a pass became
reachable from a granted run. This entry deliberately did not change with it:
what it is about is the aggregation, and the fixture's reach is unchanged.

## The ordering that was measured rather than read

A critical check that was honestly out of scope does not block, so it lands in
`critical_out_of_scope` — the clause at the end of the frozen severity ladder in
`crates/sure-domain/src/status.rs`. Reading that ladder top to bottom suggests
the clause decides and the answer is `needs_attention`.

**It does not.** The `counts.checked() == 0` clause comes first, and with an
out-of-scope skip as the only check nothing ran, so the answer is
`not_enough_checked`. Measured, not read: two calls to the same function on the
same machine, one pass apart.

| | checked | not checked | severity |
| --- | --- | --- | --- |
| the skip, alone | 0 | 1 | `not_enough_checked` |
| the same skip beside a pass | 1 | 1 | `needs_attention` |

Both halves are in the fixture as `scope_limit_alone` and
`scope_limit_beside_a_pass`, because a measurement that lives only in a README is
a sentence and not an assertion. This pins the **order of two branches**, not the
scope-limit clause itself.

**No entry here claims to pin that clause, and it must not be read as though one
did.** `counts.not_checked()` already counts a scope-limited skip, so the
`|| !critical_out_of_scope.is_empty()` clause is strictly redundant today: the
branch above it fires first and reaches the same `needs_attention`. The two
states are identical and no assertion can tell them apart.

## `needs_attention` is a ceiling for `scope_limit_beside_a_pass`

It is not a control and the fixture does not imply one. A passing check beside an
out-of-scope critical check cannot make the run green, because the project was
still not fully checked — `not_checked() > 0` fires. The acceptance's control is
`control`, and there is exactly one run in this directory that reaches `Green`.

## A name collision, recorded rather than papered over

`fixtures/adversarial/unknown-evidence/` is fully built and wired into both
corpora, and a reader who saw it in the corpus list could reasonably conclude
that `CheckStatus::Unknown` is covered. **It is not.** That fixture's "unknown"
is `ClaimAssessment::CannotConfirm` plus `StalenessReason::UnknownProvenance` —
two different enums, neither of them `CheckStatus` — and a *successful* claim
check returns no `CheckResult` at all
(`crates/sure-core/src/pipeline.rs`), so it never reaches `aggregate`. The
`Unknown` status is graded here or nowhere; `checker_failure_skipped` is the same
statement for `CheckStatus::Skipped`.

## How to see it for yourself

```
cargo test -p sure-core --test adversarial_fixture_detection -- checker_failure
```

The test reads `scenario.json`, builds each run's schedule with the product's own
`PlanBuilder`, asserts the builder refused nothing, hands each run's declared
results to `aggregate_run`, and compares every declared answer with the product's
— the exact severity, the exact headline, every row's status, state, evidence
class, reason and sentence, the coverage buckets and the counts. It writes
nothing anywhere: there is no scratch directory and no store.

## Watch out

The fixture declares **no** `entry_points`, and that is load-bearing rather than
an omission. `crates/sure-core/tests/finding_severity_rule.rs` decides which
release-blocking cases have a fixture app by looking for that key, and adding it
here would pull this case into a list it does not belong on and stop
`every_release_blocking_case_with_a_fixture_app_meets_the_manifest` by design.

If a later build stops walking the schedule, or starts answering
`not_enough_checked` because a stage could not run rather than because a check
did not, the assertion that disagrees will be one of two: the row-presence
assertion, which reads the rows out of `report.critical()` and asserts that a
check this directory declares is a row in the report at all, or the control's
severity. The `unreported()` list is held against the declared ids by the
exhaustive comparison at the end of the test, not by that assertion.

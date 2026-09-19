# repair-regression

A small shop with two departments, and one rule they both use.

This fixture is here to answer one question: **if a repair fixes the thing that
was broken and breaks something that used to work, can SURE still call it
fixed?** The answer this directory is built to give is no, and the interesting
half is the other one — that a repair which really does fix everything can still
close the finding. A checker that simply refused every repair would answer the
first half and be useless, so the fixture is built so that both halves really
happen and the checks that tell them apart really run.

## What is in here

| Where | What it is |
|---|---|
| `shared/pricing.js` | the one rule both departments use, and where the mistake is |
| `packages/checkout/` | works out what a customer pays after a discount — **wrong as shipped** |
| `packages/billing/` | works out what a customer gets back after a handling fee — **right as shipped** |
| `scripts/demo.js` | prints both answers, so you can see the mistake without reading any code |

You need nothing but Node. There is nothing to install: this fixture declares no
dependencies at all.

```
node scripts/demo.js       # both answers for one basket
node packages/checkout/tests/total.js    # the department that is wrong
node packages/billing/tests/refund.js    # the department that is right
```

Each of the last two prints one line and exits non-zero if anything is wrong.
`npm start` runs the demo, and `npm test` inside either package runs that
package's own check.

## The mistake, in one sentence

`less(total, amount)` is supposed to take `amount` off `total`. It adds it.
Both departments call the same function and only one of them gets a wrong
answer, because billing hands in the fee already negated — so the mistake
cancels out there.

```
basket of 1000 with a 250 discount: 1250 (expected 750)     <- wrong
refund of 1000 with a 100 fee:       900 (expected 900)     <- right, by accident
```

## The two repairs

**The careless one.** Correct the one line in `shared/pricing.js`:
`total + amount` becomes `total - amount`. The basket is right now. And the
refund prints 1100 instead of 900, because billing is still negating a fee that
is already being taken off. Something that used to work is now broken, and the
check that says so is `packages/billing/tests/refund.js`.

**The complete one.** Correct that line *and* the call in
`packages/billing/src/refund.js` (`less(paidCents, -feeCents)` becomes
`less(paidCents, feeCents)`), because the meaning of the shared rule moved under
the call. Both checks pass now, and both answers are right.

`scenario.json` records what SURE is supposed to do about each of the two, and
`crates/sure-core/tests/repair_fixture_e2e.rs` is the test that runs them.

## What SURE itself says about this directory

Measured at `P14-T009`, in this build, by copying the directory to
`target/tmp` and running the real program over the copy — the shipped fixture is
never run and never edited:

```
> sure check <copy-of-this-directory>
Not enough could be checked to say whether this is ready.
...
3 check(s) could not run or were skipped. 3 of them are critical.
  (start the project, run the tests in packages/billing, run the tests in packages/checkout)

  - run the tests in packages/billing   (Checking this would have meant running
    your project's code, and you have not allowed that.) [critical]
  - run the tests in packages/checkout  (Checking this would have meant running
    your project's code, and you have not allowed that.) [critical]
  - start the project                   (Checking this would have meant running
    your project's code, and you have not allowed that.) [critical]
```

That is the honest reading rather than a limitation of the fixture: SURE does not
run a project's code unless the person at the keyboard has said it may, so the
shipped directory is reported as three things it could not check rather than as
three things that passed. The third of the three is not one of this fixture's
checks — it is the probe SURE builds from the `start` script because a project
that says how to start itself is worth seeing start — and
`crates/sure-core/src/repair_impact.rs` can never select it as a regression
check, because a probe's evidence is an `ObservedFact` and that rule asks for a
`DeterministicCheck`.

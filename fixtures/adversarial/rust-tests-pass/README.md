# rust-tests-pass

## What this project claims

It claims to be a checkout's order total: a discount comes off the subtotal, and
an order never costs less than nothing.

## What is actually there

Exactly that. `total_cents` subtracts the discount:

```
subtotal_cents.saturating_sub(discount_cents)
```

All three tests in `src/lib.rs` pass, and `cargo test` exits zero. `cargo fmt
--check`, `cargo check --all-targets` and `cargo clippy --all-targets` pass too.

## Why a fixture that passes anything at all is in here

Because this product's worst failure is a false green, and the way to tell a
detector apart from a checker that refuses everything is to hand it something
that is fine and see whether it says so.

This is the control half of a pair. The other half is
`fixtures/adversarial/rust-tests-fail`: the same project with `saturating_add`
in place of `saturating_sub`. The files that make up the project —
`Cargo.toml`, `src/main.rs`, `rustfmt.toml`, `clippy.toml` — are byte for byte
the same in both halves, and `src/lib.rs` differs by that one line. `README.md`,
`scenario.json` and `scripts/check.ps1` are the only files that differ for a
reason other than the defect: each half has to say which half it is.

So the pair is an assertion in two directions at once. If SURE reports this
half as failing, not ready or needing attention, the finding it produced for
the other half is worth nothing — a checker that says no to everything has
detected nothing. And if SURE reports the other half as green, the green it
prints here is worth nothing.

## How to see it for yourself

```
powershell -File scripts/check.ps1
```

The script copies the project to a scratch directory outside the checkout and
runs `cargo test` there, because `cargo` writes `Cargo.lock` and build output
next to the manifest it reads and this project ships inside SURE's own
repository. It prints what the check said and exits with the code `cargo` gave
it, which is zero here.

You can also run the project's own check directly:

```
cargo test
```

That works too, and it leaves a `Cargo.lock` and a `target/` directory in this
project — which is why the script copies first and why the tests for this
fixture run against a copy.

## What SURE should say about it

SURE should discover the project from its `Cargo.toml`, propose the four checks
a Rust project gets, run them, and find nothing to report. Both `rustfmt.toml`
and `clippy.toml` ship here, so all four checks are proposed: a run that shows
fewer than four is a runner that dropped one, not a project that did not ask for
one. With every check run and none of them failing, the run is **green** and the
project looks ready to hand off.

Green here means *everything that could be checked passed*. It does not mean
nothing visible went wrong, and it is not a licence to skip: a check that was
never run must still leave the run short of green, even on a project that is in
fact fine.

## Watch out

Nothing in this project is broken, so there is nothing here to catch by reading
it. This directory's job is to be the thing the other half is measured against,
and a reader looking for a trap will not find one on purpose.

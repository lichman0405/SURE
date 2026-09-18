# rust-tests-fail

## What this project claims

It claims to be a checkout's order total: a discount comes off the subtotal, and
an order never costs less than nothing.

The claim is written down three times, and all three agree with each other. The
function's doc comment says the discount comes off. The three tests in
`src/lib.rs` assert it: 2500 with 500 off is 2000, an order with no discount
costs the subtotal, and a discount bigger than the order is capped at zero.

## What is actually there

The tests fail.

`total_cents` adds the discount instead of subtracting it:

```
subtotal_cents.saturating_add(discount_cents)
```

`2500` with a `500` discount comes out at `3000`. Two of the three tests fail on
that line, and `cargo test` exits non-zero.

This is not a trick and nothing is staged. The project is wrong in the ordinary
way a project is wrong, and the tool every Rust project uses to find that out
says so.

## How to see it for yourself

```
powershell -File scripts/check.ps1
```

The script copies the project to a scratch directory outside the checkout and
runs `cargo test` there, because `cargo` writes `Cargo.lock` and build output
next to the manifest it reads and this project ships inside SURE's own
repository. It prints what the check said and exits with the code `cargo` gave
it, which is non-zero here.

You can also run the project's own check directly:

```
cargo test
```

That works too, and it leaves a `Cargo.lock` and a `target/` directory in this
project — which is why the script copies first and why the tests for this
fixture run against a copy.

## Watch out

The project's own check **fails**, and that is the point of this half: it is a
real failure, not a fixture that has been made to look broken.

What it does *not* tell you is anything about the other three things SURE runs.
`cargo fmt --check`, `cargo check --all-targets` and `cargo clippy --all-targets`
all pass on this project: the line is well formatted, it compiles, and it is not
a lint. A report that only looked at whether the project builds would call this
project fine.

## What SURE should say about it

SURE should discover the project from its `Cargo.toml`, propose the four checks a
Rust project gets, run them, and report the failing one: `run the tests` is
critical, so this project is **not ready to hand off** while it fails.

SURE must **not** report this project as green, passing or ready, and it must not
treat a copy of the project as a different project: the copy is the one place the
check can run without writing into the checkout, and the tests assert that its
content fingerprint is this directory's.

## The other half

`fixtures/adversarial/rust-tests-pass` is the same project with one line
corrected — `saturating_sub` in place of `saturating_add`, and nothing else. The
files that make up the project — `Cargo.toml`, `src/main.rs`, `rustfmt.toml`,
`clippy.toml` — are byte for byte the same in both halves, and `src/lib.rs`
differs by that one line. It must come back green, and if it does not, the
failing verdict above is worth nothing: a checker that refuses everything has not
detected anything.

`README.md`, `scenario.json` and `scripts/check.ps1` are the only files that
differ for a reason other than the defect: each half has to say which half it
is.

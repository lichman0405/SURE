//! Print one order, so the project is something a person can run.
//!
//! This file is byte-for-byte the same in both halves of the pair. The defect is
//! in `src/lib.rs`, and the program below is the same program in both: the
//! failure half prints a total that is too high, which is what the defect looks
//! like from outside.

use sure_fixture_rust_order::total_cents;

fn main() {
    let subtotal_cents = 2_500;
    let discount_cents = 500;

    println!("subtotal : {subtotal_cents} cents");
    println!("discount : {discount_cents} cents");
    println!(
        "total    : {} cents",
        total_cents(subtotal_cents, discount_cents)
    );
}

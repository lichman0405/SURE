//! Order totals for a small checkout.
//!
//! One function and three tests, so that this project and the other half of its
//! pair differ by exactly one line of this file. That is the whole of the pair's
//! design: the two halves are each other's control, and a difference anywhere
//! else would leave room for the verdicts to differ for a reason nobody wrote
//! down.

/// What a customer pays for an order, in cents.
///
/// The discount comes off the subtotal. An order never costs less than nothing,
/// so a discount larger than the order is capped at zero rather than wrapping
/// around.
#[must_use]
pub fn total_cents(subtotal_cents: u64, discount_cents: u64) -> u64 {
    subtotal_cents.saturating_add(discount_cents)
}

#[cfg(test)]
mod tests {
    use super::total_cents;

    #[test]
    fn a_discount_comes_off_the_subtotal() {
        assert_eq!(total_cents(2_500, 500), 2_000);
    }

    #[test]
    fn an_order_with_no_discount_costs_the_subtotal() {
        assert_eq!(total_cents(2_500, 0), 2_500);
    }

    #[test]
    fn a_discount_larger_than_the_order_is_capped_at_nothing() {
        assert_eq!(total_cents(100, 500), 0);
    }
}

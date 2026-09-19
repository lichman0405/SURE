'use strict';

/**
 * The one helper both packages reach for.
 *
 * That is the whole of this fixture's arrangement: one edit here is felt in two
 * places, and each package's own check is what tells the difference between an
 * edit that fixed one thing and an edit that broke another.
 */

/** `total` reduced by `amount`, never below zero. */
function less(total, amount) {
  // The defect this fixture ships. `packages/checkout`'s check fails because of
  // this line, and `packages/billing`'s check passes because it negates the fee
  // it hands in — see `packages/billing/src/refund.js`. Fixing this line alone
  // is the careless repair: it fixes checkout and breaks billing.
  return Math.max(0, total + amount);
}

module.exports = { less };

'use strict';

const { less } = require('../../../shared/pricing.js');

/**
 * What the customer gets back: what they paid, less the handling fee.
 *
 * The fee is handed in negated, because `less`'s second argument is the amount
 * taken off. This one call is what a repair has to move when it changes what
 * `less` means — and it is the call a careless repair leaves alone.
 */
function refundCents(paidCents, feeCents) {
  return less(paidCents, -feeCents);
}

module.exports = { refundCents };

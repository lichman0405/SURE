'use strict';

const { less } = require('../../../shared/pricing.js');

/** What the customer pays: the basket, less the discount code they applied. */
function totalCents(subtotalCents, discountCents) {
  return less(subtotalCents, discountCents);
}

module.exports = { totalCents };

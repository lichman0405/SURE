'use strict';

// The double the checkout screen points at while no provider is wired up. It
// answers with canned data, so a developer can walk the whole flow offline and
// see the screen the shop will see.
//
// It sits in `src/` beside the client it stands in for: what tells the two
// apart is the file name, which marks this one as a hand-written double, and
// nothing here is ever shipped to a shop.

const CUSTOMER = {
  id: 'customer-1',
  email: 'test@example.com'
};

function charge(cents) {
  return Promise.resolve({ charged: cents, customer: CUSTOMER });
}

module.exports = { charge: charge, CUSTOMER: CUSTOMER };

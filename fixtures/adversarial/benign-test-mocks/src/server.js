'use strict';

// The whole service: a basket in, a charge out. Everything else in this
// directory is either the arithmetic under it or the client it talks through.

const { total } = require('./basket');
const { charge } = require('./gateway');

function handleCheckout(basket, address, send) {
  const cents = total(basket);
  if (cents === 0) {
    return Promise.resolve({ ok: false, reason: 'there is nothing to charge' });
  }
  return charge(cents, address, send).then(function (answer) {
    return { ok: true, cents: answer.charged };
  });
}

module.exports = { handleCheckout: handleCheckout };

'use strict';

// What a reviewer with nothing installed but Node runs: `npm test`. It drives
// the service the way a shop does, with a sender the checks supply, so the
// whole file works with no network and no provider account.

const assert = require('node:assert');

const { addItem, total } = require('../src/basket');
const { handleCheckout } = require('../src/server');
const { sampleBasket, answeringSender } = require('../tests/checkout');

let basket = sampleBasket();
assert.strictEqual(total(basket), 2100, 'the basket adds up');

basket = addItem(basket, { sku: 'tin', cents: 300, quantity: 1 });
assert.strictEqual(total(basket), 2400, 'a later addition is counted');

handleCheckout(
  basket,
  'https://payments.invalid/shop',
  answeringSender('{"charged":2400}')
).then(function (answer) {
  assert.strictEqual(answer.ok, true, 'a configured address charges the basket');
  assert.strictEqual(answer.cents, 2400, 'the amount charged is the whole basket');
  console.log('benign-test-mocks: every check passed');
});

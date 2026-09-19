'use strict';

// The double the checkout checks run against: a sender that answers the way a
// payment provider does, so no check below opens a socket.
//
// It is a mock in the plain sense of the word — it stands in for a provider
// while the checkout screen is being built — and none of it ships.

const { addItem } = require('../src/basket');

// A basket the checks and the worked example both start from.
function sampleBasket() {
  let basket = [];
  basket = addItem(basket, { sku: 'tea', cents: 450, quantity: 2 });
  basket = addItem(basket, { sku: 'mug', cents: 1200, quantity: 1 });
  return basket;
}

// A sender of the shape `src/gateway.js` expects: address, body, done.
function answeringSender(answer) {
  return function (address, body, done) {
    done(null, answer);
  };
}

module.exports = { sampleBasket: sampleBasket, answeringSender: answeringSender };

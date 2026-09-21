'use strict';

// The basket the checkout screen hands over. Nothing in this file talks to
// anything else, so it is the half of the service a reader can reason about on
// its own.

function addItem(items, item) {
  return items.concat([
    { sku: item.sku, cents: item.cents, quantity: item.quantity || 1 }
  ]);
}

function total(items) {
  return items.reduce(function (sum, item) {
    return sum + item.cents * item.quantity;
  }, 0);
}

module.exports = { addItem: addItem, total: total };

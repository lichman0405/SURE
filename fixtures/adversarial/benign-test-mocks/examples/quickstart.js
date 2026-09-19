'use strict';

// A worked example: fill a basket and show what it costs. Fetch it from a shop
// by requiring this file, or read it as the page the documentation links to.
//
// The address below is a placeholder, so running this example contacts nobody.

const { addItem, total } = require('../src/basket');

let basket = [];
basket = addItem(basket, { sku: 'tea', cents: 450, quantity: 2 });
basket = addItem(basket, { sku: 'mug', cents: 1200, quantity: 1 });

module.exports = {
  basket: basket,
  cents: total(basket),
  address: 'https://payments.invalid/shop'
};

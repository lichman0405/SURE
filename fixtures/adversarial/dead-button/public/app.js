'use strict';

// The "Buy now" button.
//
// This file is loaded by index.html in a browser and by scripts/check.js in
// Node, so the button's behaviour has exactly one definition.

let ordersCreated = 0;

/**
 * Do the thing the button promises.
 *
 * @param {{items: number, totalCents: number}} cart
 * @returns {undefined}
 */
function placeOrder(cart) {
  // TODO: create the order, take the payment, and return a confirmation.
  // placeholder: this returns nothing and creates nothing.
  return undefined;
}

/**
 * What happens when a customer presses Buy now.
 *
 * @returns {undefined}
 */
function handleBuyClick() {
  const confirmation = placeOrder({ items: 1, totalCents: 2500 });
  if (confirmation === undefined) {
    return undefined;
  }
  ordersCreated += 1;
  return confirmation;
}

if (typeof document !== 'undefined') {
  const buyButton = document.getElementById('buy');
  buyButton.addEventListener('click', handleBuyClick);
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    placeOrder: placeOrder,
    handleBuyClick: handleBuyClick,
    ordersCreated: function () {
      return ordersCreated;
    },
  };
}

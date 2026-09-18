'use strict';

// What the web page asks the backend for.
//
// Both paths below are written out in full, so they are exactly the paths that
// go on the wire. There is no variable and no string building to hide them.

/**
 * The customer's order list.
 *
 * The page shows "You have no orders yet" for every customer, because this
 * request never reaches a handler.
 *
 * @returns {Promise<object>}
 */
function loadOrders() {
  return fetch('/api/orders').then(function (response) {
    return response.json();
  });
}

/**
 * The service health widget in the corner of the page.
 *
 * This one does reach a handler, which is why the page looks half-alive.
 *
 * @returns {Promise<object>}
 */
function loadHealth() {
  return fetch('/api/health').then(function (response) {
    return response.json();
  });
}

module.exports = { loadOrders: loadOrders, loadHealth: loadHealth };

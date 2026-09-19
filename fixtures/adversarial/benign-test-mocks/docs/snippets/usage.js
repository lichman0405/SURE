'use strict';

// The snippet the documentation page embeds, kept in the repository so the page
// and the code it describes cannot drift apart.
//
// TODO: hold this file and `examples/quickstart.js` to the same source once the
// documentation build runs both of them.

const { total } = require('../../src/basket');

module.exports = function priceOf(basket) {
  return total(basket);
};

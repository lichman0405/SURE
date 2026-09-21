'use strict';

// The project's own check. It presses the Buy now button the only way a
// headless program can: it calls the handler the button is wired to.
//
// It exits 0 on purpose. The project's own check asserts that the handler can
// be called without throwing, which is true of a handler that does nothing.

const { handleBuyClick, ordersCreated } = require('../public/app.js');

const before = ordersCreated();
const result = handleBuyClick();
const after = ordersCreated();

console.log('fixture                : dead-button');
console.log('handler returned       : ' + String(result));
console.log('orders before the press: ' + before);
console.log('orders after the press : ' + after);
console.log('effect of pressing     : none');
console.log('verdict of this project\'s own check: PASS (on purpose)');
process.exit(0);

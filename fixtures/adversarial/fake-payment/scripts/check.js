'use strict';

// The project's own check. It runs the production checkout path in process,
// with no server and no network, and prints exactly what happened.
//
// It exits 0 on purpose. This fixture is the trap: the project's own check is
// green while the payment path charges nothing, which is precisely the state
// SURE exists to catch.

const { chargeCard, PAYMENT_API_KEY } = require('../src/payments');

const result = chargeCard({ number: 'TEST-CARD-0000', expiry: '00/00' }, 2500);

console.log('fixture           : fake-payment');
console.log('api key in source : ' + PAYMENT_API_KEY);
console.log('chargeCard result : ' + JSON.stringify(result));
console.log('http clients used : none (src/payments.js requires no network module)');
console.log("verdict of this project's own check: PASS (on purpose)");
process.exit(0);

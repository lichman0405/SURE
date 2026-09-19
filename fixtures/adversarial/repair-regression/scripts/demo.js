'use strict';

// A reviewer with nothing but Node can run this — `node scripts/demo.js`, or
// `npm start` — and see both packages' answers for one basket, so the defect
// and the regression are visible without reading a check.

const { totalCents } = require('../packages/checkout/src/total.js');
const { refundCents } = require('../packages/billing/src/refund.js');

const basket = totalCents(1000, 250);
const refund = refundCents(1000, 100);

process.stdout.write(`basket of 1000 with a 250 discount: ${basket} (expected 750)\n`);
process.stdout.write(`refund of 1000 with a 100 fee: ${refund} (expected 900)\n`);

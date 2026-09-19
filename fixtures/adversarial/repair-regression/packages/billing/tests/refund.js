'use strict';

const { check } = require('../../../shared/check.js');
const { refundCents } = require('../src/refund.js');

check([
  ['the handling fee comes off the refund', refundCents(1000, 100), 900],
  ['a refund with no fee returns what was paid', refundCents(1000, 0), 1000],
  ['a fee larger than the payment never goes below zero', refundCents(100, 250), 0],
]);

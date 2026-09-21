'use strict';

const { check } = require('../../../shared/check.js');
const { totalCents } = require('../src/total.js');

check([
  ['a discount comes off the basket', totalCents(1000, 250), 750],
  ['a basket with no discount is unchanged', totalCents(1000, 0), 1000],
  ['a discount larger than the basket never goes below zero', totalCents(100, 250), 0],
]);

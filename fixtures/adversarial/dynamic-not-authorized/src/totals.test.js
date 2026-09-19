// The project's own check. It needs no framework: this fixture declares no
// dependency, so `npm test` runs `node src/totals.test.js` directly.
const assert = require('node:assert');

const { totals } = require('./totals');

assert.strictEqual(totals([{ amount: 3 }, { amount: 4 }]), 7);
assert.strictEqual(totals([]), 0);

console.log('totals: 2 checks passed');

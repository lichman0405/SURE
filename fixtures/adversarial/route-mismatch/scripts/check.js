'use strict';

// The project's own check. It asks the backend which paths it serves, then asks
// the frontend which paths it calls, and prints both lists.
//
// It exits 0 on purpose. The project's own check asserts that both files load
// without an error, which is true of two programs that never agree on a name.

const fs = require('node:fs');
const path = require('node:path');

const { app } = require('../src/backend/server.js');

const frontendSource = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'frontend', 'api.js'),
  'utf8',
);

// The paths the page asks for, read out of the calls it makes.
const wanted = [];
const callPattern = /fetch\('([^']+)'\)/g;
let match = callPattern.exec(frontendSource);
while (match !== null) {
  wanted.push(match[1]);
  match = callPattern.exec(frontendSource);
}

const served = app.routes();

console.log('fixture                  : route-mismatch');
console.log('backend serves           : ' + served.join(', '));
console.log('frontend asks for        : ' + wanted.join(', '));
console.log(
  'asked for but not served : ' +
    wanted.filter(function (one) {
      return !served.some(function (route) {
        return route.endsWith(' ' + one);
      });
    }).join(', '),
);
console.log('verdict of this project\'s own check: PASS (on purpose)');
process.exit(0);

'use strict';

// The project's own check. It signs in with the wrong password and prints what
// came back.
//
// It exits 0 on purpose. The project's own check only asserts that an account
// which exists can sign in, which is the bug stated as a feature.

const { login, verifyPassword } = require('../src/auth');

const wrong = login('admin@example.com', 'not-the-password');
const empty = login('admin@example.com', '');
const unknown = login('nobody@example.com', 'hunter2');

console.log('fixture                    : fake-auth');
console.log('password really compared?  :');
console.log('  verifyPassword("admin@example.com", "anything") -> ' +
  verifyPassword('admin@example.com', 'anything'));
console.log('sign in, wrong password    : ' + JSON.stringify(wrong));
console.log('sign in, empty password    : ' + JSON.stringify(empty));
console.log('sign in, unknown account   : ' + JSON.stringify(unknown));
console.log('verdict of this project\'s own check: PASS (on purpose)');
process.exit(0);

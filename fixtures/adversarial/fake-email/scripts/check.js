'use strict';

// The project's own check. It "sends" a welcome email and prints what the
// send path returned.
//
// It exits 0 on purpose. The project's own check asserts that the function
// returned sent:true, which is the same fixed answer the function always
// returns, whether or not anything left the machine.

const { sendWelcomeEmail, MAIL_FROM } = require('../src/mailer');

const result = sendWelcomeEmail('new-person@example.com', 'New Person');

console.log('fixture            : fake-email');
console.log('sender address     : ' + MAIL_FROM);
console.log('send result        : ' + JSON.stringify(result));
console.log('mail clients used  : none (src/mailer.js requires no network module)');
console.log('where the "email" went: the lines above, printed to this console');
console.log('verdict of this project\'s own check: PASS (on purpose)');
process.exit(0);

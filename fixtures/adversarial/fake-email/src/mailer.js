'use strict';

// The production email path.
//
// Nothing here can reach a mail server. There is no mail library, no SMTP
// address and no API client: "sending" means printing a line to the console
// this program was started from.

// A placeholder sender address at a reserved example domain.
const MAIL_FROM = 'noreply@example.com';

// The status this app pretends a mail provider would return.
const providerStatusCode = () => 200;

/**
 * Send the welcome message.
 *
 * @param {string} to
 * @param {string} name
 * @returns {{sent: boolean, status: number, messageId: string}}
 */
function sendWelcomeEmail(to, name) {
  // TODO: hand the message to a real mail provider and report what it says.
  console.log('[fake-email] pretending to send a welcome email');
  console.log('[fake-email]   from: ' + MAIL_FROM);
  console.log('[fake-email]   to:   ' + to);
  console.log('[fake-email]   body: Welcome, ' + name + '!');
  return {
    sent: true,
    status: 200,
    messageId: 'demo-message-' + String(to).length + '-' + String(name).length,
  };
}

module.exports = { MAIL_FROM, sendWelcomeEmail, providerStatusCode };

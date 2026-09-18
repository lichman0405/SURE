'use strict';

// A local sign-up server, so the fixture can be looked at in a browser.
// Plain Node, no packages: `npm start` needs nothing installed but Node.

const http = require('node:http');

const { sendWelcomeEmail } = require('./mailer');

const HOST = '127.0.0.1';
const PORT = Number(process.env.PORT || 4313);

function readBody(request, done) {
  let body = '';
  request.on('data', (chunk) => {
    body += chunk;
  });
  request.on('end', () => done(body));
}

const server = http.createServer((request, response) => {
  if (request.method === 'GET' && request.url === '/') {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(
      [
        '<!doctype html><html lang="en"><meta charset="utf-8">',
        '<title>Sign up (adversarial fixture)</title>',
        '<body style="font-family:system-ui;margin:3rem;max-width:32rem">',
        '<h1>Create your account</h1>',
        '<form method="post" action="/api/signup">',
        '<p><input name="email" value="new-person@example.com"></p>',
        '<p><input name="name" value="New Person"></p>',
        '<p><button>Sign up</button></p>',
        '</form>',
        '<p>The next page says a welcome email was sent.</p>',
        '</body></html>',
      ].join('\n'),
    );
    return;
  }

  if (request.method === 'POST' && request.url === '/api/signup') {
    readBody(request, (body) => {
      const parsed = JSON.parse(body || '{}');
      const result = sendWelcomeEmail(parsed.email, parsed.name);
      response.writeHead(200, { 'content-type': 'application/json' });
      response.end(
        JSON.stringify({ account: 'created', emailSent: result.sent }),
      );
    });
    return;
  }

  response.writeHead(404, { 'content-type': 'application/json' });
  response.end(JSON.stringify({ error: 'not found' }));
});

server.listen(PORT, HOST, () => {
  console.log('fake-email fixture listening on http://' + HOST + ':' + PORT + '/');
  console.log('Signing up will print the "sent" email to this console.');
});

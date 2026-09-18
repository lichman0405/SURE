'use strict';

// A local sign-in server, so the fixture can be looked at in a browser.
// Plain Node, no packages: `npm start` needs nothing installed but Node.

const http = require('node:http');

const { login } = require('./auth');

const HOST = '127.0.0.1';
const PORT = Number(process.env.PORT || 4312);

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
        '<title>Sign in (adversarial fixture)</title>',
        '<body style="font-family:system-ui;margin:3rem;max-width:32rem">',
        '<h1>Sign in</h1>',
        '<p>Account <code>admin@example.com</code>, any password.</p>',
        '<form method="post" action="/api/login">',
        '<p><input name="email" value="admin@example.com"></p>',
        '<p><input name="password" value=""></p>',
        '<p><button>Sign in</button></p>',
        '</form></body></html>',
      ].join('\n'),
    );
    return;
  }

  if (request.method === 'POST' && request.url === '/api/login') {
    readBody(request, (body) => {
      const parsed = JSON.parse(body || '{}');
      const result = login(parsed.email, parsed.password);
      response.writeHead(result.ok ? 200 : 401, {
        'content-type': 'application/json',
      });
      response.end(JSON.stringify(result));
    });
    return;
  }

  response.writeHead(404, { 'content-type': 'application/json' });
  response.end(JSON.stringify({ error: 'not found' }));
});

server.listen(PORT, HOST, () => {
  console.log('fake-auth fixture listening on http://' + HOST + ':' + PORT + '/');
  console.log('Sign in as admin@example.com with an empty password.');
});

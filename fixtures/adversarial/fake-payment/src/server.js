'use strict';

// A local checkout server. Plain Node, no packages: `npm start` works on a
// machine with nothing installed but Node itself.

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const { chargeCard } = require('./payments');

const HOST = '127.0.0.1';
const PORT = Number(process.env.PORT || 4311);

function readBody(request, done) {
  let body = '';
  request.on('data', (chunk) => {
    body += chunk;
  });
  request.on('end', () => done(body));
}

const server = http.createServer((request, response) => {
  if (request.method === 'GET' && request.url === '/') {
    const page = fs.readFileSync(
      path.join(__dirname, '..', 'public', 'checkout.html'),
      'utf8',
    );
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(page);
    return;
  }

  if (request.method === 'POST' && request.url === '/api/checkout') {
    readBody(request, (body) => {
      const parsed = JSON.parse(body || '{}');
      const result = chargeCard(parsed.card || {}, parsed.amountCents || 0);
      response.writeHead(result.status, { 'content-type': 'application/json' });
      response.end(
        JSON.stringify({ paid: result.ok, reference: result.reference }),
      );
    });
    return;
  }

  response.writeHead(404, { 'content-type': 'application/json' });
  response.end(JSON.stringify({ error: 'not found' }));
});

server.listen(PORT, HOST, () => {
  console.log('fake-payment fixture listening on http://' + HOST + ':' + PORT + '/');
  console.log('The page will say the payment succeeded.');
  console.log('No payment provider is contacted, ever.');
});

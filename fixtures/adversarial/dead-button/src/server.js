'use strict';

// Serves the two static files so the button can be pressed in a browser.
// Plain Node, no packages: `npm start` needs nothing installed but Node.

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const HOST = '127.0.0.1';
const PORT = Number(process.env.PORT || 4314);

const FILES = {
  '/': ['index.html', 'text/html; charset=utf-8'],
  '/app.js': ['app.js', 'text/javascript; charset=utf-8'],
};

const publicDir = path.join(__dirname, '..', 'public');

const server = http.createServer((request, response) => {
  const entry = FILES[request.url];
  if (request.method !== 'GET' || entry === undefined) {
    response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    response.end('not found');
    return;
  }
  const body = fs.readFileSync(path.join(publicDir, entry[0]));
  response.writeHead(200, { 'content-type': entry[1] });
  response.end(body);
});

server.listen(PORT, HOST, () => {
  console.log('dead-button fixture listening on http://' + HOST + ':' + PORT + '/');
  console.log('Press "Buy now". The button works; the order is never created.');
});

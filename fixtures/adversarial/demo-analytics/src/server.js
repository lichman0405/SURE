'use strict';

// A local dashboard, so the fixture can be looked at in a browser.
// Plain Node, no packages: `npm start` needs nothing installed but Node.

const http = require('node:http');

const { liveMetrics, chartSeries, recordPageView } = require('./metrics');

const HOST = '127.0.0.1';
const PORT = Number(process.env.PORT || 4315);

function page() {
  const metrics = liveMetrics();
  return [
    '<!doctype html><html lang="en"><meta charset="utf-8">',
    '<title>Dashboard (adversarial fixture)</title>',
    '<body style="font-family:system-ui;margin:3rem;max-width:40rem">',
    '<h1>Dashboard <span style="color:#0a0">&#9679; Live</span></h1>',
    '<p>Active users: <b>' + metrics.activeUsers + '</b></p>',
    '<p>Revenue: <b>$' + (metrics.revenueCents / 100).toFixed(2) + '</b></p>',
    '<p>Signups: <b>' + metrics.signups + '</b></p>',
    '<p>Updated: <b>' + metrics.updatedAt + '</b></p>',
    '<p>Weekly series: ' + chartSeries().join(', ') + '</p>',
    '</body></html>',
  ].join('\n');
}

const server = http.createServer((request, response) => {
  if (request.method === 'GET' && request.url === '/') {
    recordPageView('/');
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(page());
    return;
  }

  response.writeHead(404, { 'content-type': 'application/json' });
  response.end(JSON.stringify({ error: 'not found' }));
});

server.listen(PORT, HOST, () => {
  console.log('demo-analytics fixture listening on http://' + HOST + ':' + PORT + '/');
  console.log('The dashboard says "Live". It is reading a constant.');
});

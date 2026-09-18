'use strict';

// The order service.
//
// It serves two paths. The web page asks for a third one, which is the defect
// this fixture exists for.

const express = require('./mini-express');

const HOST = '127.0.0.1';
const PORT = Number(process.env.PORT || 4316);

const app = express();

app.get('/api/health', function (request, response) {
  response.json({ ok: true });
});

// Note the singular. The frontend asks for /api/orders.
app.get('/api/order', function (request, response) {
  response.json({ orders: [] });
});

if (require.main === module) {
  app.listen(PORT, HOST, function () {
    console.log('route-mismatch fixture listening on http://' + HOST + ':' + PORT + '/');
    console.log('Serving: ' + app.routes().join(', '));
  });
}

module.exports = { app: app };

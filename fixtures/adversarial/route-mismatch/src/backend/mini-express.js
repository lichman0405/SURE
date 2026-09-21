'use strict';

// A stand-in for Express.
//
// This fixture must run on a machine with nothing installed but Node, so the
// app object it builds is a short implementation of the four Express calls this
// project uses: get, post, listen and a res that can send JSON.

const http = require('node:http');

function express() {
  const routes = [];

  function record(method) {
    return function (path, handler) {
      routes.push({ method: method, path: path, handler: handler });
      return app;
    };
  }

  const app = {
    get: record('GET'),
    post: record('POST'),

    routes: function () {
      return routes.map(function (route) {
        return route.method + ' ' + route.path;
      });
    },

    listen: function (port, host, ready) {
      const server = http.createServer(function (request, response) {
        const route = routes.find(function (candidate) {
          return (
            candidate.method === request.method &&
            candidate.path === request.url
          );
        });
        if (route === undefined) {
          response.writeHead(404, { 'content-type': 'application/json' });
          response.end(JSON.stringify({ error: 'not found' }));
          return;
        }
        const res = {
          status: function (code) {
            response.statusCode = code;
            return res;
          },
          json: function (body) {
            response.setHeader('content-type', 'application/json');
            response.end(JSON.stringify(body));
            return res;
          },
        };
        route.handler({ method: request.method, url: request.url }, res);
      });
      return server.listen(port, host, ready);
    },
  };

  return app;
}

module.exports = express;

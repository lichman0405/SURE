'use strict';

// The payment client. The address is configuration rather than a literal: this
// service is deployed for more than one shop and the endpoint is not the same
// in any two of them.

const { request } = require('node:https');

// The way this module sends a request by default. It is a parameter everywhere
// below so that a caller can hand in a sender of its own, which is what the
// checks and the example do rather than opening a socket.
function sendOverHttps(address, body, done) {
  const call = request(address, { method: 'POST' }, function (answer) {
    let text = '';
    answer.on('data', function (chunk) {
      text += chunk;
    });
    answer.on('end', function () {
      done(null, text);
    });
  });
  call.on('error', done);
  call.end(body);
}

function charge(cents, address, send) {
  const chosen = address || process.env.GATEWAY_URL;
  if (!chosen) {
    return Promise.reject(new Error('no gateway address is configured'));
  }
  const sender = send || sendOverHttps;
  return new Promise(function (resolve, reject) {
    sender(chosen, JSON.stringify({ cents: cents }), function (error) {
      if (error) {
        reject(error);
        return;
      }
      resolve({ charged: cents });
    });
  });
}

module.exports = { charge: charge, sendOverHttps: sendOverHttps };

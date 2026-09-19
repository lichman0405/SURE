'use strict';

// A stand-in for the pricing service: this stub answers the same shape the real
// service does, so the worked example above runs with no network and no account.
//
// It lives under `stubs/` rather than beside the client because it is not a
// double for a module in this repository — it is the reply a service gives.

function quote(cents, quantity) {
  return { cents: cents * quantity };
}

module.exports = { quote: quote };

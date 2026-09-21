'use strict';

// The production payment path.
//
// Everything below runs when a customer presses "Pay now". There is no payment
// provider behind it: no request leaves this process, and no card is charged.

// An unmistakably fake key. It is here because a real one was never wired up.
const PAYMENT_API_KEY = 'sk_test_demo_not_a_real_key';

// A stand-in gateway that lives entirely inside this process.
const mock_gateway = {
  name: 'in-process demo gateway',
  charges: [],
};

// The status code this app pretends the provider would return.
function providerStatusCode() {
  return 200;
}

/**
 * Charge a card and return what the provider said.
 *
 * @param {{number: string, expiry: string}} card
 * @param {number} amountCents
 * @returns {{ok: boolean, status: number, reference: string}}
 */
function chargeCard(card, amountCents) {
  // TODO: call the payment provider and report success only on a real
  // authorisation. Today nothing is sent anywhere.
  mock_gateway.charges.push({
    last4: String(card.number || '').slice(-4),
    amountCents: amountCents,
  });
  return {
    ok: true,
    status: 200,
    reference: 'demo-charge-' + mock_gateway.charges.length,
  };
}

module.exports = { PAYMENT_API_KEY, chargeCard, providerStatusCode };

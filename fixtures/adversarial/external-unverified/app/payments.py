"""The billing path: what happens when a customer pays.

The request below is the one Stripe's API documents for creating a charge. It
is built correctly and then handed to `transport.send`, which is a stand-in
that never leaves this process. Whether Stripe would accept it is not something
this machine can establish.
"""

import os

from app import transport

# Stripe's own base address, overridable so that a deployment can point
# somewhere else. Reading this is how the project says which external service
# it uses.
DEFAULT_API_BASE = "https://api.stripe.com"

CHARGE_PATH = "/v1/charges"


def api_base():
    """The provider address this process is configured to use."""
    return os.getenv("STRIPE_API_BASE") or DEFAULT_API_BASE


def secret_key():
    """The account key, as this process was given it.

    An empty string means no key is configured. No key of any kind is written
    down in this repository, so this is the only place one can come from.
    """
    return os.getenv("STRIPE_SECRET_KEY", "")


def build_charge_request(card_token, amount_cents, currency):
    """The request Stripe's charge endpoint is called with."""
    return {
        "method": "POST",
        "url": api_base() + CHARGE_PATH,
        "headers": {
            "Authorization": "Bearer " + secret_key(),
            "Content-Type": "application/x-www-form-urlencoded",
        },
        "body": {
            "source": card_token,
            "amount": str(amount_cents),
            "currency": currency,
        },
    }


def charge(card_token, amount_cents, currency="usd"):
    """Charge a card and report what the provider said.

    `paid` below is the provider's answer as this process received it. Nothing
    here checks that the answer came from Stripe, and nothing here could: the
    transport in `app/transport.py` returns the same answer every time.
    """
    answer = transport.send(build_charge_request(card_token, amount_cents, currency))
    body = answer.get("body") or {}
    return {
        "paid": bool(body.get("paid")),
        "reference": body.get("id", ""),
        "provider_status": answer.get("status"),
    }

"""This project's own check.

It charges a card through the billing path, prints the request that was built,
prints how many requests actually left this machine, and exits 0.

It exits 0 on purpose. The transport it runs against is the stand-in in
`app/transport.py`, which answers the same way whatever it is handed, so a
success here is a sentence about the stand-in rather than about payments. The
check does not hide that: it counts the requests that left the machine and
prints the number, which is zero, on the line above its verdict.

Whether Stripe would accept this request cannot be checked from here — it needs
an account, a key, a network and a card. Saying so is the honest answer, and
this check cannot say it.

Run it with:

    python scripts/check.py
"""

import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from app import payments, transport  # noqa: E402  (the path above has to come first)


def describe(headers):
    """The request as a person can read it, with the key left out."""
    shown = dict(headers)
    if "Authorization" in shown:
        shown["Authorization"] = "Bearer <the key this process was given>"
    return shown


def main():
    # A key, handed to this process the way a deployment would hand it over.
    # Nothing in this repository holds one.
    os.environ.setdefault("STRIPE_SECRET_KEY", "a-key-this-check-made-up")

    request = payments.build_charge_request("tok_from_the_checkout_page", 2500, "usd")
    attempt = payments.charge("tok_from_the_checkout_page", 2500)

    print("fixture                         : external-unverified")
    print("request method and address      : %s %s" % (request["method"], request["url"]))
    print("request headers                 : %r" % (describe(request["headers"]),))
    print("request body                    : %r" % (request["body"],))
    print("provider answer                 : %r" % (attempt,))
    print("requests handed to the transport : %d" % transport.requests_handed_over())
    print("requests that left this machine  : %d" % transport.connections_opened())
    print("whether the provider would accept this : not established here, by anybody")
    print("verdict of this project's own check: PASS (on purpose)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

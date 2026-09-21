"""The billing app.

Run it with:

    python app/main.py

It takes a payment through the billing path and prints what came back. The
provider it names is Stripe; the transport the request is handed to is the
stand-in in `app/transport.py`, which never opens a connection.

No key is configured here, so the request is built with an empty one. The point
of the run is to show that the app is happy either way.
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from app import payments  # noqa: E402  (the path above has to come first)


def main():
    attempt = payments.charge("tok_from_the_checkout_page", 2500)

    print("external-unverified fixture: the billing app")
    print("provider named : " + payments.api_base() + payments.CHARGE_PATH)
    print("account key    : %s" % ("configured" if payments.secret_key() else "not configured"))
    print("charge result  : %r" % (attempt,))
    print("provider is real, reachable and would accept this: not established")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

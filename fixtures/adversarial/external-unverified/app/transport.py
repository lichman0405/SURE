"""Where a request to the payment provider would go.

The project is written as though a request leaves this machine and reaches
Stripe. It does not. `send` is a stand-in: it reads the request it is handed,
writes down that it arrived, and returns a prepared answer.

Nothing here opens a connection, therefore nothing here can fail for the reason
a real integration fails — a rejected key, a declined card, an account without
the right permissions. A deployment would replace `send` with one that talks to
Stripe; nothing in this project does, and this project contains no code that
could tell the difference.
"""

# Every request this process handed over, in the order it was handed over.
HANDED_OVER = []

# What the stand-in answers with.
PREPARED_RESPONSE = {
    "status": 200,
    "body": {"id": "ch_stand_in_0001", "paid": True},
}


def send(request):
    """Hand a request to the provider and get its answer back.

    The answer is a constant in this file. It does not depend on the key, the
    amount, the card or the account, which is why nothing downstream can use it
    to tell a working integration from a broken one.
    """
    HANDED_OVER.append(request)
    return dict(PREPARED_RESPONSE)


def requests_handed_over():
    """How many requests this process gave to the stand-in."""
    return len(HANDED_OVER)


def connections_opened():
    """How many connections to a provider this process opened.

    A constant rather than a counter, because there is no code path in this
    module that opens one: the only import above is a comment, and the only
    thing `send` does is append to a list. A deployment's transport would count
    a real number here, and this one has nothing to count.
    """
    return 0

"""The configuration this app reads out of its environment.

Every name below is read when the app starts, and every one of them is
required. None of them is named anywhere the project ships: there is no
`env`-style sample file in this repository, and the README does not list them.

So the only way to learn what this application needs is to read this file, or
to start the application and be told that something is absent.
"""

import os

# The names the application asks for, in the order it asks for them. This list
# is for reporting; the reads themselves are below, each spelled out at the
# line that makes it.
REQUIRED = [
    "ORDERS_DB_URL",
    "ORDERS_QUEUE_URL",
    "REPORTING_BUCKET",
]


def orders_database_url():
    """Where the orders are kept. Required: there is no default."""
    return os.environ["ORDERS_DB_URL"]


def orders_queue_url():
    """Where finished orders are announced. Required: there is no default."""
    return os.environ["ORDERS_QUEUE_URL"]


def reporting_bucket():
    """Where the nightly report is written. Required in practice: an empty
    answer means the report has nowhere to go."""
    return os.getenv("REPORTING_BUCKET", "")


def missing():
    """The required names this process was not given, in the order above.

    This is the only place in the project that answers the question, and it
    answers it about the environment rather than about any file. Nothing in the
    project says what the values should look like either.
    """
    return [name for name in REQUIRED if not os.getenv(name)]

"""The orders app.

Run it with:

    python app/main.py

It builds a database from the model, writes an order, reads it back and prints
what it found. That is the whole application: a fresh database every time.

What it does not do is touch a database that already exists. That is the half
of the project the missing migration was for, and it is not written.
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from app import store  # noqa: E402  (the path above has to come first)


def main():
    connection = store.connect()
    store.create_schema(connection)
    store.add_order(connection, "ORD-1001", 2500, 500)

    print("missing-migration fixture: the orders app")
    print("database   : created empty, in memory, from app/models.py")
    print("columns    : " + ", ".join(store.table_columns(connection, "orders")))
    print("order      : %r" % (store.find_order(connection, "ORD-1001"),))
    print("this run never opened an existing database.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

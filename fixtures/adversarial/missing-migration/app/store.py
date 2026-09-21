"""The storage layer: the one place in this project that talks to a database.

SQLite comes with Python, so this runs on a machine with nothing installed.
"""

import sqlite3

from app.models import ORDER_COLUMNS


def connect():
    """A connection to an empty database, in memory.

    SQLite's in-memory database is created when the connection is made and
    thrown away when it closes, so this never writes a file anywhere.
    """
    return sqlite3.connect(":memory:")


def create_schema(connection):
    """Create the orders table from the model.

    Every column the model lists is created here. This is why the app works on
    a fresh database and why the missing migration is invisible from here: the
    table is built from the current model, so it always has the current shape.
    """
    columns = ", ".join("%s %s" % (name, kind) for name, kind in ORDER_COLUMNS)
    connection.execute("CREATE TABLE orders (%s)" % columns)
    connection.commit()


def table_columns(connection, table):
    """The columns a table really has, as the database reports them."""
    rows = connection.execute("PRAGMA table_info(%s)" % table).fetchall()
    return [row[1] for row in rows]


def add_order(connection, reference, total_cents, discount_cents):
    """Write one order."""
    connection.execute(
        "INSERT INTO orders (reference, total_cents, discount_cents) VALUES (?, ?, ?)",
        (reference, total_cents, discount_cents),
    )
    connection.commit()


def find_order(connection, reference):
    """Read one order back, or nothing when there is no such order."""
    row = connection.execute(
        "SELECT reference, total_cents, discount_cents FROM orders WHERE reference = ?",
        (reference,),
    ).fetchone()
    return row

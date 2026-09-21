"""The shape this project says its database has.

The list below is the current shape: it is what the code in this project
expects to find when it opens the orders table. It is not the shape a database
created before the discount change actually has, and nothing in this project
turns one into the other.
"""

from dataclasses import dataclass


# Every column an order has today, in creation order.
#
# `discount_cents` arrived with the discount change. The change was made in
# this file and nowhere else: the revision that would add the column to a
# database that already exists was never written, so `alembic/versions/` is
# empty while this list is not.
ORDER_COLUMNS = [
    ("id", "INTEGER PRIMARY KEY AUTOINCREMENT"),
    ("reference", "TEXT NOT NULL"),
    ("total_cents", "INTEGER NOT NULL"),
    ("discount_cents", "INTEGER NOT NULL DEFAULT 0"),
]

# What the orders table looked like before the discount change. It is here so
# that the difference is readable rather than remembered.
COLUMNS_BEFORE_THE_DISCOUNT_CHANGE = [
    ("id", "INTEGER PRIMARY KEY AUTOINCREMENT"),
    ("reference", "TEXT NOT NULL"),
    ("total_cents", "INTEGER NOT NULL"),
]


@dataclass(frozen=True)
class Order:
    """One order, as the application works with it."""

    reference: str
    total_cents: int
    discount_cents: int

    def payable_cents(self):
        """What the customer actually owes."""
        return self.total_cents - self.discount_cents

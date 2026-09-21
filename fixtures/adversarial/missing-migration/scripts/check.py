"""This project's own check.

It exercises the order path against a database it builds from the model, prints
what happened, and exits 0.

It exits 0 on purpose, and the reason is worth stating rather than leaving in
the code: it builds a *fresh* database every run, so it can only ever see the
current model. The defect this fixture is about is not in the model. It is in
the revision that was never written beside it, and a check that creates its own
database from scratch can never see a missing one.

The check does not hide that. It counts the revisions in `alembic/versions/`
and prints the number, which is zero, on the line above its verdict. The number
is the finding; the verdict is still PASS.

Run it with:

    python scripts/check.py
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from app import store  # noqa: E402  (the path above has to come first)


def revisions():
    """The revision files Alembic would run, by name."""
    versions = ROOT / "alembic" / "versions"
    if not versions.is_dir():
        return []
    return sorted(path.name for path in versions.glob("*.py"))


def main():
    connection = store.connect()
    store.create_schema(connection)
    store.add_order(connection, "ORD-1", 2500, 500)
    found = store.find_order(connection, "ORD-1")

    print("fixture                       : missing-migration")
    print("database this check used      : created empty a moment ago, in memory")
    print("columns it built              : " + ", ".join(store.table_columns(connection, "orders")))
    print("order it read back            : %r" % (found,))
    print("revisions in alembic/versions : %d" % len(revisions()))
    print("databases that already exist  : not opened, not read, not migrated")
    print("verdict of this project's own check: PASS (on purpose)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

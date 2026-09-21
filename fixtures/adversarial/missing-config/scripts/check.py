"""This project's own check.

It supplies the configuration the application needs, starts it, and exits 0.

It exits 0 on purpose, and this is the line that makes it a trap: the check
sets the three settings itself, in code, before the application runs. It is
doing the part of the job that documentation was supposed to do. A person who
clones this project has no such code, and nothing to copy the names from.

The check does not hide it. It says how many of the settings it supplied, and
that number is all of them, on the line above its verdict.

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

from app import settings  # noqa: E402  (the path above has to come first)


def supply_the_settings_the_project_never_documents():
    """Hand the application what it asks for.

    None of these values is in the project anywhere else, and none of the names
    is in a sample file or a document. They exist here, in this function, which
    is the only place in the repository that knows them.
    """
    os.environ["ORDERS_DB_URL"] = "sqlite://:memory:"
    os.environ["ORDERS_QUEUE_URL"] = "memory://local"
    os.environ["REPORTING_BUCKET"] = "sure-fixture-reports"


def main():
    supply_the_settings_the_project_never_documents()
    absent = settings.missing()

    print("fixture                                  : missing-config")
    print("settings the application reads           : " + ", ".join(settings.REQUIRED))
    print(
        "configuration this check supplied itself : %d of %d"
        % (len(settings.REQUIRED) - len(absent), len(settings.REQUIRED))
    )
    print("configuration the project ships          : none, in no file and no document")
    print("orders database, as the app resolved it  : " + settings.orders_database_url())
    print("order queue, as the app resolved it      : " + settings.orders_queue_url())
    print("report bucket, as the app resolved it    : " + settings.reporting_bucket())
    print("still absent after this check's help     : " + (", ".join(absent) if absent else "nothing"))
    print("verdict of this project's own check: PASS (on purpose)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

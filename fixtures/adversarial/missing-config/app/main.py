"""The reporting app.

Run it with:

    python app/main.py

It reports what configuration it was given and what it is missing. On a machine
where none of the names is set — which is how a new machine looks — all three
are missing, and the project offers nothing to copy them from.

It exits 0 either way. Reporting that it cannot start is something this program
can do; starting is not.
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from app import settings  # noqa: E402  (the path above has to come first)


def main():
    absent = settings.missing()

    print("missing-config fixture: the reporting app")
    print("settings it reads : " + ", ".join(settings.REQUIRED))
    for name in settings.REQUIRED:
        print("  %-18s : %s" % (name, "given" if name not in absent else "not given"))
    print("nothing in this project names any of them outside the code that reads them.")

    if absent:
        print("this run did not start: %d setting(s) are absent" % len(absent))
    else:
        print("this run started.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

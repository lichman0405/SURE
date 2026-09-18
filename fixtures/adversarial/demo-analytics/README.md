# demo-analytics

## What this project claims

It claims to be a dashboard. Next to the title there is a green dot and the word
**Live**. It shows active users, revenue, signups and a chart for the week, and
it says it was updated *a moment ago*.

## What is actually there

None of those numbers are measured. They are typed into a source file, and they
are the same every time anybody looks. The "a moment ago" is typed in as well.

The tracking that is supposed to feed these figures does not happen either:
recording a page view prints a line on the console, and a signup is recorded
against a made-up placeholder user. The tracking id in the code was never
registered with any analytics service, so nothing anywhere is collecting
anything.

## How to see it for yourself

```
node scripts/check.js
```

It reads the "live" figures twice and prints both. They are identical, and the
script notes that the module reads no database, file or service.

Or, to use the page:

```
node src/server.js
```

Open <http://127.0.0.1:4315/>, reload it as often as you like, and watch the
numbers refuse to move. Then look at the terminal: the page views the dashboard
claims to be tracking are being printed there and thrown away.

## Watch out

The project's own check **passes**. It asks whether there are numbers to show,
and a constant always has numbers. A green tick from this project says nothing
about whether the dashboard is honest.

This one is less urgent than the other fixtures. Nothing here can lose money or
let the wrong person in, so the fixture is not release-blocking. It is still a
lie a customer would read.

## What SURE should say about it

SURE should tell you, in plain words, that the "Live" label is not true: that
the figures are fixed values written into the code, that nothing is recorded
when a page is viewed or an account is created, and that the tracking id belongs
to nobody.

SURE must **not** report the dashboard as showing live data, and it must not
report the project as having working analytics. SURE must also not swing the
other way and complain about ordinary numbers: a genuine constant such as a
retry count is not a defect. What is wrong here is a constant presented as a
measurement.

# route-mismatch

## What this project claims

It claims to be a page that lists your orders. It has a small health indicator
in the corner that says the service is fine, and then it shows your order list.

## What is actually there

The page and the service disagree about one word.

The page asks the service for `/api/orders` — plural. The service has only ever
been told about `/api/order` — singular. So the page's request lands nowhere,
and the page shows "You have no orders yet" to every customer, whether they
have orders or not.

The health indicator works, because that path does match. That is what makes
this one hard to spot by hand: half the page behaves, so it looks alive.

## How to see it for yourself

```
node scripts/check.js
```

It asks the service which paths it serves, reads the paths out of the page's
requests, and prints both lists side by side. The line that matters is "asked
for but not served".

Or, to see it on the wire:

```
node src/backend/server.js
```

Then ask for the path the page asks for, and for the one the service actually
has. The first is refused, the second answers.

## Watch out

The project's own check **passes**. It only asks whether both files load without
an error, which two programs that never agree on a name will happily do.

## What SURE should say about it

SURE should tell you, in plain words, that the page asks for something the
service does not offer: that `/api/orders` is called and never served, so the
empty order list a customer sees is not the truth about their orders. It should
name both files and the line each one is on.

SURE must **not** report the order list as working, and it must **not** report
`/api/health` as a mismatch. That one is declared, and calling it a mismatch
would be a false alarm that teaches people to ignore the real one.

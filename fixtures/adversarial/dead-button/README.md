# dead-button

## What this project claims

It claims to be a shop with one item in the basket and a **Buy now** button. It
claims that pressing the button places your order.

## What is actually there

The button is not broken. You can press it, and it does respond. It calls the
piece of code that is supposed to place the order, and that piece of code does
nothing at all: it creates no order, takes no payment, saves nothing, and does
not even complain. It just returns "nothing" and stops.

The button is wired to a dead end. The screen never changes.

## How to see it for yourself

```
node scripts/check.js
```

It presses the button the way a program can — by calling the handler the button
is wired to — and prints how many orders exist before and after. Both numbers
are zero.

Or, to use the page:

```
node src/server.js
```

Open <http://127.0.0.1:4314/> and press **Buy now**. The line under the button
changes to *Still waiting for the order to be placed…*, and it waits forever.
Nothing was ordered.

## Watch out

The project's own check **passes**. It only asks whether the button can be
pressed without the program falling over. A button that does nothing passes
that check easily.

## What SURE should say about it

SURE should tell you, in plain words, that the most important button on the page
has nothing real behind it: that pressing it creates no order, that the code
calls itself a placeholder, and that a customer would see nothing happen.

SURE must **not** report this project as working, complete or passing, and it
must not treat the presence of a click handler as proof that the button works.

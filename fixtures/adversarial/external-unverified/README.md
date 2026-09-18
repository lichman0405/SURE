# external-unverified

## What this project claims

It claims to take card payments through Stripe.

It has a billing module, it reads a Stripe API key and a Stripe API base from
the environment, it builds the request Stripe's own API documents, and it hands
back a charge identifier and a success. The checkout page would say the payment
worked.

## What is actually there

Nothing verifies that it does.

The request is built and then given to a transport that lives inside this
process. That transport records the request and hands back a prepared response.
It opens no connection, sends nothing, and has never been run against Stripe.

Whether a real Stripe account would accept this request is not a question this
machine can answer. It needs an account, a key with the right permissions, a
network, and a card — none of which is here, and none of which SURE has.

The project's own check is in the same position. It runs the billing path with
the in-process transport, sees a success, and has no way to tell that success
apart from a real one.

## How to see it for yourself

```
python scripts/check.py
```

It charges a card through the billing path, prints the request that was built,
prints how many requests left the machine, and exits 0.

That count is the interesting line, and the check prints it in plain sight:

```
requests that left this machine  : 0
```

Or run the app itself:

```
python app/main.py
```

It does the same thing through the same transport: a payment with no network
behind it.

## Watch out

The project's own check **passes**, and it passes for a reason that has nothing
to do with payments. Its transport is a stand-in, so "the charge succeeded" is
a sentence about the stand-in.

A green tick from this project does not mean a card was charged, does not mean
Stripe was contacted, and does not mean the integration works. It means two
functions in one process agreed with each other.

## What SURE should say about it

SURE should report the payment path as **not checked**, in plain words: that
this project uses an external payment service, that the request is built for
Stripe, and that whether Stripe accepts it cannot be confirmed here. The honest
answer is "cannot confirm", not "pass" and not "fail".

SURE must **not** report the payment path as working, verified or passing on
the strength of the in-process transport's response — not on the strength of
the fixture's own passing check either.

# fake-auth

## What this project claims

It claims to have a sign-in page. You type your email address and your password,
and it lets you in or turns you away.

## What is actually there

The password is not checked. The program looks up whether an account with that
email address exists, and if it does, it lets you in — whatever you typed in the
password box, including nothing at all.

The code says so in a comment: the password arrives, is never compared to
anything, and is thrown away.

The accounts it lets you into are placeholders, not real people. One of them is
`admin@example.com` with the password `hunter2`. Those values were made up for
the fixture and are not credentials for anything.

## How to see it for yourself

```
node scripts/check.js
```

It signs in with an obviously wrong password and with an empty one, and prints
what came back. Both succeed.

Or, to use the page:

```
node src/server.js
```

Open <http://127.0.0.1:4312/>, leave the password box empty, and press
**Sign in**. You are in.

## Watch out

The project's own check **passes**. It only asks whether an account that exists
can sign in. That is exactly the behaviour that is wrong, so a green tick from
this project tells you nothing.

## What SURE should say about it

SURE should tell you, in plain words, that this sign-in does not check
passwords: that any password opens an account that exists, that the accounts are
placeholders, and that the author knew and left a note rather than finishing the
work.

SURE must **not** report this project as working, secure, verified or passing,
and it must not treat the fixture's own passing check as proof that sign-in is
safe.

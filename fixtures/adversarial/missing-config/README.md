# missing-config

## What this project claims

It claims to be a reporting service that a person can set up and run.

It has a configuration module, it reads what it needs from the environment, and
it will tell you when something is absent. The setup story reads as though the
names it needs are known.

## What is actually there

Nothing tells you what those names are.

The application requires three settings. There is no sample configuration file
in the project, no documentation listing them, and no defaults to fall back on.
The only place the names appear at all is inside the code that reads them.

So a person who clones this project cannot start it. They will be told that
something is missing, one name at a time, by a program that will not run without
it. Nothing in the project says what the values should look like, either.

The project's own check never meets this problem, because it hands the
application the three names itself, in code, before it starts anything.

## How to see it for yourself

```
python app/main.py
```

It reports which settings it was given and which are absent. Run it in a shell
with none of them set — which is how a new machine looks — and all three are
absent.

Then run the project's own check:

```
python scripts/check.py
```

It sets the three names in the process it is about to run, then starts the
application, which works. The line worth reading is the one above its verdict:

```
configuration this check supplied itself : 3 of 3
```

## Watch out

The project's own check **passes**. It passes because it supplies the very
configuration the project never documents; the check is doing the part of the
job the missing documentation was for.

That is the whole trap. A green tick from this project means the code runs when
somebody already knows what to set. It does not mean anyone else can run it, and
it is not evidence that the settings are written down anywhere.

## What SURE should say about it

SURE should tell you, in plain words, which names the source files ask the
environment for, and that no sample configuration file and no document in the
project names any of them — so the person setting this project up next has
nothing to follow.

There is no case for this fixture in the release manifest, so it does not block
a release on its own. SURE should still report the mismatch rather than
aggregate the project to green.

SURE must **not** report the project as set up, working or verified on the
strength of the fixture's own check passing.

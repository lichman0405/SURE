# missing-migration

## What this project claims

It claims to be an orders service whose database is managed properly.

The order table gained a column — a discount — and the model file says so. The
project has an `alembic.ini`, so it looks like the database shape is managed by
Alembic the way every Alembic project manages it.

## What is actually there

The migration was never written.

`alembic/versions/` is in the project and holds no revision at all. Nothing
says how a database that already exists gets the new column. A database created
before the change keeps the old shape forever, and the code that reads a
discount from it has nothing to read.

The model file, `app/models.py`, lists the current shape. The migration
directory says how to get there from the old one, and it is empty. Those two
are supposed to agree, and here one of them is missing.

## How to see it for yourself

```
python scripts/check.py
```

The check builds a database from the model in memory, writes an order, reads it
back, and prints the columns it got. It also prints how many revisions
`alembic/versions/` holds.

That number is the interesting one, and the check prints it in plain sight:

```
revisions in alembic/versions : 0
```

Or run the app itself:

```
python app/main.py
```

It builds a fresh database from the model every time, so it works. A database
that already exists is not something either command looks at.

## Watch out

The project's own check **passes**. It passes whenever the model is
self-consistent, which it is — the defect is not in the model, it is in the
migration that was never written beside it.

The check creates its database from scratch on every run, so it can only ever
see the current shape. That is exactly the shape the missing migration was
supposed to give an existing database. A green tick from this project says the
model is consistent with itself. It says nothing about any database that was
created before the change.

## What SURE should say about it

SURE should tell you, in plain words, that `alembic.ini` says this project
manages its database with Alembic, that `alembic/versions/` is present and
holds no migration, and that nothing in the project records how the database
reached the shape `app/models.py` describes.

SURE must **not** report this project as working, verified or passing, and it
must not treat the fixture's own passing check as proof that the database can
be brought up to date.

# The privacy corpus

`manifest.json` is the data; `crates/sure-cli/tests/privacy_suite.rs` is what runs
it. Together they are the referent for a word three documents were using without
one: `docs/testing/ADVERSARIAL_FIXTURES.md` ("A mandatory false green blocks
release."), `docs/product/DEFINITION_OF_DONE.md` ("secret-redaction fixtures
pass;") and `docs/product/PRODUCT_EVALS.md` ("secret redaction mandatory
fixtures").

```
cargo test -p sure-cli --test privacy_suite
cargo test -p sure-cli --test privacy_suite -- --nocapture   # prints the inventory and the NOT CHECKED list
```

## What it is, and what it is not

It is not a set of scenario directories under `fixtures/adversarial/`. Its
entries are cases about a **run**: what a real `sure` process left on disk when
there was no settings file at all, what a hook answered when a project's file
named a mode this build cannot read, what a refusal printed and what it refused
to print. Those are claims about a process and a file, and they are checked by
starting the process and reading the file.

Every entry carries six things a reader can enumerate:

| field | what it is |
| --- | --- |
| `id` | the name the suite fails by |
| `kind` | `hook_ingest`, `check`, `full_recording_on_disk`, or `covered_by` |
| `layer` | where the claim is observed: `binary`, `store`, or `test` |
| `release_blocking` | whether a failure of it blocks a release |
| `why` | why this entry is here rather than somewhere else |
| `promised_by` | the documents that promise it, and the sentence in each |

`promised_by` is how "mandatory" has a referent. The suite fails when a document
stops making a sentence an entry is bound to, so a withdrawn promise takes its
entry with it instead of leaving a fixture nobody promised and a promise nobody
tests. The sentence is matched word for word and not whitespace for whitespace: a
document is prose that gets reflowed, and a binding that broke on every rewrap is
a binding somebody deletes.

## The entries that point somewhere else

`kind: covered_by` means the behaviour is already proved by a test at the layer
this corpus would prove it at, and the entry names the file and the tests instead
of re-running them. `every_pointer_names_a_test_that_exists` reads each named file
and fails when a named test is not in it, so the pointer cannot decay into prose:
rename or delete the test and the corpus fails.

Two of these deserve a note.

- `the-default-run-is-the-one-no-test-is-named-for-but-two-walks` — the round-trip
  tests in `crates/sure-cli/src/hook.rs` already run with no settings file of any
  kind and already assert that no recording was stored. They are named for the
  round trip, not for the default, which is exactly why the default is also its
  own driven case here: a behaviour nothing is named after disappears in a rename.
- `the-three-acts-an-allowance-reaches-are-holds-and-only-those` — the broad
  change, the force push and the read of credentials are held by a rule that is
  tested, but the *scenario directories* for those three acts belong to `P14-T008`
  ("Delete/force-push/sensitive-read expected protection behavior tested"). See
  `not_confirmed[the-dangerous-action-scenarios-are-p14-t008s]`.

## What "no false green" means in this file

- A case asserts an **absence beside a presence**. "Nothing was recorded" proves
  nothing on its own — a run that failed, or that wrote no store at all, would
  satisfy it. So `no-settings-file-at-all-records-no-full-recording` requires a
  stored `event` row before the missing recording means anything, and
  `a-secret-typed-into-a-goal-reaches-the-store-only-redacted` requires the
  redacted text to be in the store before the absent credential means anything.
- Every case reads **every byte** under its store directory, not the value a
  reader is handed back: a redaction is a claim about the file, and a row read
  back through a redacting path would pass while the page still held the secret.
- A case that cannot run is not a case that passed. The holes are in
  `not_confirmed`, they are printed by
  `the_corpus_prints_what_it_could_not_confirm`, and the field a reader is looking
  for is `would_settle_it`.
- Every case says whether it blocks a release, and a `covered_by` entry says which
  layer the test it points at observes at.

## The premise the recording cases run under

`no_settings_file_at_all_is_the_premise_the_binary_cases_run_under` asserts, in
both directions, that this machine has no user settings file: this process checks
the path through `Paths`, and `sure doctor` reports the same path absent. A
machine whose owner has turned full recording on cannot answer "what does SURE do
by default", and the suite fails there rather than passing on a premise that is
not true.

## What could not be confirmed

Printed on every run. Two gaps and one seam:

- **`the-report-shows-a-goal-the-history-does-not-hold`** — `sure check --goal`
  prints the goal verbatim under a sentence about the record, and the record holds
  the redacted form. The corpus asserts the store half and records the report
  half, because the honest expectation is not settled: `PROJECT_INTENT.md` says
  the text is stored minus nothing, and the store redacts every document it
  accepts. One of the two rules has to win, and that is a product decision.
- **`no-recording-through-the-binary`** — no case here opens a full recording
  through a real process. The consent comes only from the user's own settings
  file, which on Windows is `%APPDATA%\SURE\sure.yaml` and is not redirectable:
  `--store-dir` moves the store and not the settings, and no test may write the
  settings of the person running the suite. The recording case is therefore at the
  `store` layer, through the same two calls the hook makes.
- **`the-dangerous-action-scenarios-are-p14-t008s`** — the seam with that task.

## Adding an entry

1. Add it to `manifest.json` with an `id`, one of the four kinds, a `layer`,
   `release_blocking`, a `title`, a `why`, and at least one `promised_by`.
2. If it is `covered_by`, name the `file` and the `tests`. Nothing else is needed:
   the suite checks the pointer.
3. If it is driven, write its `expect`, and make sure the expectation is about
   something the binary can be observed to do. A case whose premise is a user
   settings file this machine does not have is a case that cannot run — put it in
   `not_confirmed` instead, or drive it in-process and say so in `layer_note`.
4. Run the suite. `every_case_carries_what_the_corpus_promises_a_reader_can_find`
   rejects an entry whose expectations are missing, so an entry added
   half-finished fails rather than passing quietly.

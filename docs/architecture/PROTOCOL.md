# The wire protocol

Everything SURE writes down about a project or an event — to local storage, to a
machine-readable output stream, or to a coding harness — is one of seven
documents, and each has a JSON Schema in `schemas/`. This document is the
authority for how those two halves relate. The prose form of one of them, the
harness event, is `docs/architecture/EVENT_PROTOCOL.md`.

Two things SURE writes are deliberately not in that set, and the qualifier above
is why the sentence is worth reading carefully. The first is the stored record,
which wraps a document with the version, the project and the time
(`docs/architecture/STORAGE_AND_DATA_PATHS.md` §The store); the second is the
response frame a CLI command answers in
(`docs/architecture/CLI.md` §The frame). A document is a *statement about a
project*, and those two are the containers one travels in. They carry
`protocol_version` rather than a schema of their own for the same reason: a
reader has to be able to tell which build wrote the thing holding the document
before it can decide whether it can read the document.

## The documents

| Schema | The document | Rust type | Direction |
| --- | --- | --- | --- |
| `event.schema.json` | one normalized harness event | `sure_protocol::event::EventEnvelope` | in |
| `finding.schema.json` | a material problem or uncertainty | `sure_domain::vocabulary::Finding` | out |
| `claim.schema.json` | an agent statement and its assessment | `sure_domain::vocabulary::Claim` | out |
| `check-result.schema.json` | the outcome of one check | `sure_domain::status::CheckResult` | out |
| `project-intent.schema.json` | **one** requirement, with its source | `sure_domain::intent::Requirement` | out |
| `repair.schema.json` | repair instructions for a harness | `sure_domain::vocabulary::RepairContract` | out |
| `fixture-expectation.schema.json` | what an evaluation fixture requires | — (P14-T001) | test |

Two rows need a sentence more than the table gives them.

**Project intent describes the element, not the container.** `ProjectIntent` is a
list of requirements and has no source of its own — it is the union of its
requirements' sources. The schema describes one requirement, and serializing the
container against it fails on the missing `id`, `source` and `text`. That is
correct: the container is not what the schema is about, and adding those three
fields to it so that one document could serve both would give the container a
source it does not have.

**Fixture expectation has no Rust type yet.** `docs/architecture/FROZEN_SEMANTICS.md`
records this as conformance gap 2: the schema exists, and not every
`fixtures/adversarial/*/scenario.json` file matches it yet — the ones written
before `P14-T001` do not. P14-T001–T011 resolves it. There is a test in `crates/sure-protocol/tests/conformance.rs`
naming the gap, so that adding the type means deleting the test deliberately.

## Versions

`PROTOCOL_VERSION` is the version of the **event** document, and it is the value
written into every envelope's `schema_version`. It is currently `1`.

The other six documents carry no version. They are SURE's own records, written
and read by the same build, and the version that matters for them is the
**storage** version — which belongs on the stored record and not on the document.
P1-T005 owns that decision. This is the reason there is an asymmetry below.

A version changes when the format changes in a way an older reader would get
*wrong*. Adding an optional field does not; removing a field, renaming one, or
reinterpreting one does.

## The handshake

A caller that is about to send events asks SURE what it speaks before it sends
anything: `sure protocol --speaks VERSION`, or `sure_protocol::handshake::negotiate`
for a caller written in Rust. Both go through one function, and so does
`EventEnvelope::from_json` when it refuses a document — which is the point.
Written twice, the two would drift, and the drift would be invisible: each would
keep answering its own question correctly while an adapter that the CLI told
"yes" was refused at its first event. That failure looks like a broken adapter,
and sends somebody looking in the wrong place.

**The rule is exact equality.** This build reads one version: `PROTOCOL_VERSION`,
its own. A version changes exactly when an older reader would get the format
wrong, so a mismatch means a document that would be misread, and a misread event
becomes wrong evidence about a project. There is no "still close enough" range,
and there is no table of older versions this build can still read — there has
only ever been one version, and a compatibility table with a single row is a
policy invented to look thorough. When a second version exists, the list arrives
with it, and `crates/sure-protocol/src/handshake.rs` is where it goes.

A mismatch says **which side has to move**, because the two directions are not
the same answer and a caller that has just been refused will otherwise try the
fix that cannot work:

| Caller speaks | What happens | Who changes |
| --- | --- | --- |
| this build's version | agreed | nobody |
| an older version | refused, status 3 | the caller — a newer SURE will not take it either |
| a newer version | refused, status 3 | SURE — this build cannot be taught it from here |

The sentence is `Handshake`'s `Display` and exists once. The machine form
carries `sure_speaks`, `caller_speaks`, `agreed` and `update` (which side moves)
rather than the sentence, because a script that read prose would break the first
time the prose was improved. `crates/sure-cli/src/report.rs` builds that frame
from the same value the sentence is printed from.

## Reading an event

`EventEnvelope::from_json` does four things in this order, and the order is what
makes the error message the right one:

1. Is it JSON at all? A syntax error is the sender's bug.
2. Which format version is it? Checked **before** the shape, so a document from a
   newer SURE is reported as a version mismatch. Checking the shape first would
   report it as a broken adapter, and send someone looking for the wrong
   problem.
3. Does it match `event.schema.json`? Every violation at once, in SURE's own
   words, each naming the value rather than only the rule.
4. Does it deserialize? A failure here means the schema and the Rust type
   disagree, which is SURE's mistake, and it is reported as a distinct error for
   that reason.

## Closed and open

`event.schema.json` sets `additionalProperties: false`. The other six do not.

**The event is closed** because it is the one document SURE receives from
outside. An adapter sending a field SURE does not understand must be a visible
error: ignoring it would leave SURE reporting a capability it had silently
dropped on the floor, and the report would read as "the harness cannot do that"
rather than "SURE did not look".

**The other six are open** because the domain types carry SURE's own provenance —
`fingerprint`, `session`, `next_step`, `recheck` — that a reader of the document
does not need. Forbidding the extra fields would mean either losing that
information or duplicating every type into a wire form and a storage form that
would then drift. What the schemas pin is the part a reader depends on.

## The validator

`crates/sure-protocol/src/schema.rs` is a JSON Schema checker covering exactly
the keywords the schemas use: `$schema`, `title`, `description`, `type` (single
or union), `required`, `properties`, `items`, `enum`, `minimum`, `format` and
`additionalProperties`.

A general-purpose validator was the first choice and was rejected on cost:
`jsonschema` pulls in an HTTP client, a TLS implementation and roughly fifty
transitive crates to evaluate those eleven keywords.
`docs/architecture/RUST_DESIGN.md` asks that the dependency tree stay small
enough for native Windows packaging to stay tractable, and a network stack inside
a local-first tool is the wrong shape even as a test-only dependency.

The risk of a hand-written checker is the obvious one, and two properties answer
it:

1. **An unsupported keyword is an error, not a skip.** `Schema::parse` walks the
   whole document — including `properties` and `items` — and refuses a schema
   containing anything it does not enforce. A schema cannot silently outgrow the
   validator; the parse fails and CI stops. A constraint that is quietly skipped
   reads exactly like one that passed, which is the failure this rule exists to
   prevent.
2. **The schema is not treated as an annotation.** Draft 2020-12 makes `format`
   annotation-only by default, which means a conforming validator may ignore it.
   That is a difference that makes a reader believe a timestamp was checked when
   it was not, so `format: date-time` is enforced here, and any other format
   value is refused at parse time. Only `minimum` is supported, not `maximum`;
   bounds are expressed as `enum` where they need to be exact, which is how
   `capability_tier` is written.

The schemas are compiled into the binary with `include_str!` rather than read
from disk. An installed `sure.exe` has no `schemas/` directory next to it, and a
document that could only be validated from a checkout would be validated in CI
and nowhere else. It also means an installation is always validating against the
schema its own code was built from.

## What keeps this honest

| Claim | Test |
| --- | --- |
| Every structure matches its schema | `crates/sure-protocol/tests/conformance.rs`, `every_structure_matches_the_schema_that_describes_it` |
| The validator can fail | same file, `every_required_key_is_actually_enforced_for_every_document` and `a_wrongly_typed_required_field_is_refused_for_every_document` |
| A new schema cannot go unchecked | same file, `every_document_in_the_registry_is_covered_by_this_test_or_excused` |
| Every document survives a read | `crates/sure-protocol/tests/round_trip.rs` |
| The handshake and the event reader refuse the same versions | `crates/sure-protocol/src/event.rs`, `the_reader_and_the_handshake_refuse_the_same_versions` |
| A refused caller is told which side has to move | `crates/sure-protocol/src/handshake.rs`, `a_mismatch_says_which_side_has_to_move` |
| The version SURE reports is the one it will talk to | `crates/sure-cli/tests/cli_contract.rs`, `the_protocol_version_sure_reports_is_the_one_it_will_talk_to` |
| Wire names are frozen | `crates/sure-domain/tests/wire_contract.rs` |
| The version and the schema agree | `crates/sure-protocol/src/lib.rs`, `the_version_is_the_one_the_schemas_advertise` |

The second row is the one to keep. A conformance test that always passed would
be a false green inside the check that exists to prevent false greens, so every
document is also checked with a required key removed and the test fails unless a
violation comes back. Reverting a fix and watching the tests fail is how that was
verified, not an argument that it must work.

## Two gaps this resolved

Both were found by writing the conformance test, and both had been invisible
because only the *enum names* in the schemas were checked before.

1. **`RepairContract` named its issue `issue`.** `repair.schema.json` requires
   `issue_id`, and `docs/architecture/REPAIR_PROTOCOL.md` says "issue ID". The
   field is now `issue_id`, so the Rust name and the wire name are the same word
   rather than being related by a `serde` attribute a reader has to find.
2. **`CheckResult` had no `evidence_class` and no `project_fingerprint`.** Both
   are required by `check-result.schema.json`, and both are load-bearing:
   `docs/architecture/EVIDENCE_MODEL.md` requires a statement about a project to
   declare how it was established, and a result that does not name its project
   state can be read against a later one — which is how a stale pass becomes a
   false green. `CheckResult::not_run` and `CheckResult::errored` set
   `EvidenceClass::Unknown` and take no class parameter, because a check that did
   not run established nothing and offering a choice would only offer a way to
   write that down wrongly.

`docs/architecture/FROZEN_SEMANTICS.md` records the third, the event envelope
drift between `EVENT_PROTOCOL.md` and `event.schema.json`, which this also
resolved: the schema now defines `capability_tier` and `project_root`.

## Known gaps

1. **The six stored documents have no version.** A field written by a newer SURE
   is dropped silently on read. This is asserted by a test in
   `round_trip.rs` so that it is a recorded decision rather than an accident of
   serde's defaults. P1-T005 owns the fix, because the version belongs on the
   stored record.
2. **Two of the three adapters do not exist yet.** The handshake rule is
   implemented and reachable — `sure protocol --speaks VERSION`, and the event
   reader refusing through the same function — but a caller running
   `sure hook` or the MCP server has nothing to run yet, because those commands
   are not implemented. See `docs/architecture/CLI.md` §`sure protocol` for what
   the CLI half does today.

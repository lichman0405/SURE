# The command line

`sure` is the universal surface: every user can run every check without
installing a harness plugin (`docs/adr/0004-thin-harness-integrations.md`), and
a harness that wants an active tool surface calls these commands rather than
reimplementing them.

P1-T008 built the framework: what SURE accepts, and how a result reaches a
person and a script. Two commands do their work. The rest are recognised, and
say so.

## The commands

| Command | What it will do | In this build |
| --- | --- | --- |
| `sure check [PATH]` | check a project and report what it found | recognised, not implemented |
| `sure recheck [PATH]` | check again and compare with last time | recognised, not implemented |
| `sure repair [PATH]` | turn what was found into instructions an agent can act on | recognised, not implemented |
| `sure history [list\|show\|delete\|export]` | show what SURE has recorded | recognised, not implemented |
| `sure doctor` | report where SURE keeps its files on this machine, and what it found there | recognised, not implemented |
| `sure config [paths\|show\|validate]` | show the settings in effect and which layer each came from | recognised, not implemented |
| `sure hook ingest` | record one event from a coding harness | recognised, not implemented |
| `sure explain [ID]` | explain one recorded result in plain language | recognised, not implemented |
| `sure protocol` | print the harness protocol version this build speaks | works |
| `sure version` | print the version of this build | works |

"Recognised, not implemented" is not a euphemism for a stub. The command parses
its arguments, decides it cannot do its job, **exits with status 3**, and says in
one sentence what it did not do. Nothing is half-done and nothing is reported as
done.

Where a command is scheduled is `tasks/tasks.json`. That file is not quoted here
on purpose: a phase number in a message a user reads is a promise the command
cannot keep, and one command in the table above (`sure history`) has no owning
task at all. A refusal that names a phase would be inventing one there.

### Arguments arrive with the phase that can act on them

No command carries a flag that nothing yet honours. A grammar that accepts
`--deep` and ignores it teaches the user that SURE understood them when it did
not, and the output gives them no way to find out otherwise. This is the same
rule as `additionalProperties: false` on the event schema: an input SURE does
not act on must be a visible error, not a dropped one.

A path is accepted where the documented form has one, and defaults to the
current directory. An identifier is accepted where a command names something.
Both are optional in this build, because making one required is a decision about
the command's contract that belongs to the phase implementing it.

## Two output paths, and no third

| `--format` | Stream | Shape |
| --- | --- | --- |
| `human` (default) | stdout for an answer, stderr for a complaint | prose for a person |
| `json` | always stdout | one object on one line |

`--format` is global, so it is accepted before or after the command name.

The rule that matters is not that there are two renderers. It is that there is
no third way out: `crates/sure-cli/src/output.rs` is the only module in the
crate that names a process stream, and
`crates/sure-cli/tests/cli_contract.rs::only_the_output_module_writes_to_a_stream`
reads the sources and fails if another one appears. That is a source scan
rather than a test of behaviour because what is being ruled out is an
*absence* — a stray `println!` is invisible until the day somebody pipes the
output into a JSON reader, and by then it is in a release.

A command's answer is prose on stdout, which is what makes `sure check >
report.txt` put the report in the file. A complaint goes to stderr, so the same
redirection leaves it where the person can see it. The machine form always goes
to stdout, including when the command failed: the object *is* the result, and a
script that asked for the reason has to be able to read it.

### The frame

```json
{"command":"doctor","does":"…","exit_code":3,"instead":"…","outcome":"unavailable","protocol_version":1,"sure_version":"0.0.0-bootstrap"}
```

Four fields are always present: `sure_version` and `protocol_version`, so that a
captured response says which build and which protocol produced it — those are
the two things that make a stored answer unreadable, and a response that names
neither invites the reader to assume the current ones — and `command` and
`outcome`, which are what a reader switches on.

`outcome` is a closed set: `ok`, `unavailable`. It grows when a command can end
another way, and every reader's switch has to grow with it.

**This frame is not one of the seven documents in
`docs/architecture/PROTOCOL.md`.** Those are statements about a project or an
event, each with a JSON Schema in `schemas/`; this is the envelope a command
answers in. `docs/architecture/PROTOCOL.md` carries the same note, because a
sentence there says that everything SURE writes is one of the seven, and a CLI
response is written.

The frame lives in `sure-cli` rather than in `sure-protocol` because it has one
consumer. `sure-protocol` holds contracts with things outside this repository,
and generalising a shape before a second caller needs it is how a contract
becomes a guess — which is the reasoning ADR 0012 used to turn down a logging
framework. When P12-T009's MCP server needs the same envelope, it moves, with a
real second consumer in hand.

## Exit statuses

| Status | Meaning | Who returns it |
| --- | --- | --- |
| 0 | the command did what it says it does | `version`, `protocol`, `--help`, `--version` |
| 1 | the command ran, and the answer is not clear | reserved; `sure check` once it can check |
| 2 | the command line was wrong | the parser, including a bare `sure` |
| 3 | the command exists, and this build cannot carry it out | everything in the table above marked "not implemented" |
| 4 | SURE declined, and can say why in the user's terms | reserved; configuration authority and path rules |
| 5 | the command tried and did not finish | anything, including a result that could not be written out |

The whole table is in `crates/sure-cli/src/report.rs` as `report::exit`, and it
is the only place a status is chosen. `main` handles clap's parse error itself
rather than letting `clap::Parser::parse` exit the process, because a table
where one row is a library's decision is a table that quietly becomes wrong.

Two statuses are reserved before anything returns them. A script written against
this release should not have to be rewritten because a later one reused a number
for something else.

**1 and 3 are not the same and must not be merged.** "The project has problems"
and "SURE cannot do that here" need opposite responses from the person reading
them, and one status for both is how a tool that is broken gets read as a
project that is clean.

### A bare `sure` is not a success

It did nothing, so status 0 would let a script that invoked the wrong thing read
it as a clean run. It is status 2, the same as any other wrong command line.

## Why clap

`docs/architecture/RUST_DESIGN.md` names it, and it earns its place on a surface
this user-visible: usage errors that name the flag, `--help` that cannot drift
from the grammar the parser actually accepts, and the `--` handling that decides
whether a path beginning with a dash is a flag or a path. Hand-rolling that is
where "an unknown flag was accepted and ignored" comes from, which is this
program's version of a false green.

What clap decides is what the command line means. What it does not decide is
anything about output or status: both are SURE's, and both have one home.

## What keeps this honest

| Claim | Test |
| --- | --- |
| Every documented command parses | `crates/sure-cli/tests/cli_contract.rs`, `every_documented_command_parses` |
| A command that did nothing never exits 0 | same file, `no_command_that_did_nothing_reports_success` |
| The two paths use the streams the table above names | same file, `a_refusal_reads_on_the_terminal_and_leaves_standard_output_empty` and `the_machine_form_is_one_object_on_one_line_of_standard_output` |
| Both paths describe the same outcome | same file, `the_two_paths_disagree_about_nothing_that_matters` |
| No module outside `output.rs` writes to a stream | same file, `only_the_output_module_writes_to_a_stream` |
| `sure hook ingest` does not read standard input | same file, `hook_ingest_does_not_read_standard_input` |
| The grammar is the list in this document | `crates/sure-cli/src/main.rs`, `the_grammar_is_the_one_docs_architecture_cli_md_lists` |
| A new command cannot ship with no answer about whether it works | the exhaustiveness of `Command::report` — see below |

The last row has no test, because it does not need one: `Command::report` and
`Command::name` both match every variant with no fallback arm, so adding a
command to the grammar stops the build until somebody decides. That was checked
by adding one and watching the build fail in both places, not argued.

## What this document does not cover

- **What a check finds.** `docs/architecture/CHECK_PIPELINE.md` and, when it
  exists, the report format.
- **What a hook receives.** `docs/architecture/EVENT_PROTOCOL.md` and
  `docs/integrations/`.
- **What `sure protocol` should print for a handshake.** `PROTOCOL.md` assigns
  the handshake to P1-T010; this task made the command exist.
- **Interactive prompting.** Nothing prompts yet. `sure history delete` will
  need a non-interactive path for scripts, and that is its own decision.

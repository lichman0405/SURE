# The command line

`sure` is the universal surface: every user can run every check without
installing a harness plugin (`docs/adr/0004-thin-harness-integrations.md`), and
a harness that wants an active tool surface calls these commands rather than
reimplementing them.

P1-T008 built the framework: what SURE accepts, and how a result reaches a
person and a script. P1-T009 added the first command whose answer depends on
what it found. Three commands do their work. The rest are recognised, and say
so.

## The commands

| Command | What it will do | In this build |
| --- | --- | --- |
| `sure check [PATH]` | check a project and report what it found | recognised, not implemented |
| `sure recheck [PATH]` | check again and compare with last time | recognised, not implemented |
| `sure repair [PATH]` | turn what was found into instructions an agent can act on | recognised, not implemented |
| `sure history [list\|show\|delete\|export]` | show what SURE has recorded | recognised, not implemented |
| `sure doctor` | report where SURE keeps its files on this machine, and what it found there | works |
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

## `sure doctor`

The first command whose answer depends on what it found, and therefore the first
place where the status carries news rather than the fact that SURE ran.

It reports the four paths SURE uses on this machine — evidence and history, the
settings directory, the settings file, and the record store — what is at each of
them, what is in the store if there is one, whether `git` is on `PATH`, and
everything wrong that it found. It also prints **what it did not check**, always,
because a report that stops at what it found invites the reader to assume it
looked everywhere.

Three things it deliberately does not do, each of which is what makes the report
safe to paste into a bug report:

- **It does not read the settings file.** It reports the path and whether
  something is there. `docs/architecture/DIAGNOSTICS.md` requires that a secret
  never have to be handled in order to be reported, and the way to keep that true
  is never to open the file that may hold one. Reading settings is `sure config
  show`.
- **It does not create the store.** A store that is not there is reported as
  nothing recorded yet. A diagnostic that changes what it is diagnosing is worse
  than one that says it did not look.
- **It does not run a program.** Finding a program on `PATH` is an observed fact
  about the filesystem; running it is a different claim, and SURE has no process
  runner yet. This is one of the entries under "What this did not check", so a
  reader cannot mistake one for the other.

`sure doctor` is also the first command whose status is not the same on every
machine: 0 when it found nothing wrong, 1 when it did. That is the false-green
rule applied to SURE's own installation — `sure doctor || fix it` has to work.

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

A complaint:

```json
{"command":"check","does":"…","exit_code":3,"instead":"…","outcome":"unavailable","protocol_version":1,"sure_version":"0.0.0-bootstrap"}
```

A report:

```json
{"command":"doctor","details":{"build":{…},"not_checked":[…],"places":{…},"problems":[…],"store":{…},"tools":[…]}},"exit_code":0,"outcome":"ok","protocol_version":1,"sure_version":"0.0.0-bootstrap"}
```

Five fields are always present: `sure_version` and `protocol_version`, so that a
captured response says which build and which protocol produced it — those are
the two things that make a stored answer unreadable, and a response that names
neither invites the reader to assume the current ones — `command` and `outcome`,
which are what a reader switches on, and `exit_code`, which is the status the
process returned. The last was first written only for a complaint, on the idea
that a field should appear when it says something; that is the wrong test for
this one, because a frame where it is sometimes absent is a frame a script has
to special-case. The status is a fact about every run.

`outcome` is a closed set: `ok`, `not_green`, `unavailable`. It grows when a
command can end another way, and every reader's switch has to grow with it.

`outcome` and `exit_code` are one decision written twice: `Report::outcome` and
`Report::exit_code` read the same predicate, so a build where they disagree is
not a build that can happen. They are both here because they are read by
different things — `outcome` is what a person's script switches on in the body,
and the status is what a caller acts on without parsing anything.

`not_green` says the answer is not a clean one — `sure doctor` on a machine
where SURE found something wrong with its own files. That is still an *answer*:
it goes to stdout, so `sure doctor > report.txt` puts the report in the file and
the exit status carries the bad news on its own. A complaint is SURE saying it
could not do the thing at all, and that goes to stderr.

Anything a command found goes under `details`, one key, written by that command's
own module. The five fields above then keep meaning exactly what this section
says they mean, whatever a command has to report.

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
| 0 | the command did what it says it does | `version`, `protocol`, `doctor` when it found nothing wrong, `--help`, `--version` |
| 1 | the command ran, and the answer is not a clean one | `doctor` when it found something wrong; `sure check` once it can check |
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

One code for two meanings is right where a caller does the same thing with both.
`sure check` finding problems and `sure doctor` finding a damaged store are both
"stop and read the report": the report says which, and the status is what stops
the script.

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
| A doctor report's status, outcome and problems agree | same file, `a_doctor_report_is_an_answer_however_it_turns_out` |
| No module outside `output.rs` writes to a stream | same file, `only_the_output_module_writes_to_a_stream` |
| `sure hook ingest` does not read standard input | same file, `hook_ingest_does_not_read_standard_input` |
| The grammar is the list in this document | `crates/sure-cli/src/main.rs`, `the_grammar_is_the_one_docs_architecture_cli_md_lists` |
| The doctor's labels and values line up | `crates/sure-cli/src/doctor.rs`, `every_label_the_report_prints_is_separated_from_its_value` |
| `sure doctor` never reads the settings file | `crates/sure-core/tests/doctor.rs`, `the_diagnostic_never_reaches_for_the_settings_module` |
| `sure doctor` never creates the store | same file, `the_report_never_creates_what_it_reports_on` |
| The doctor names the files the store writes | same file, `the_diagnostic_names_the_files_the_store_actually_uses` |
| The report always states what it did not check | same file, `the_report_always_says_what_it_did_not_look_at` |
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
- **How `sure doctor` decides what a problem is.** That is
  `crates/sure-core/src/doctor.rs`, which says what each state means and why a
  fresh installation is not one of them.
- **Interactive prompting.** Nothing prompts yet. `sure history delete` will
  need a non-interactive path for scripts, and that is its own decision.

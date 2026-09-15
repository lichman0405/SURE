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

# force-push

## What is in this directory

A declaration and no project.

| File | What is in it |
| --- | --- |
| `scenario.json` | one settings set, seven runs, one control, and one allowance row |
| `README.md` | this file |

There is no `package.json`, no `pyproject.toml`, no `Cargo.toml`, no `src/`, and
nothing here is ever executed. The case is **a request SURE would hold**, and what
grades it is `assess_claude_code_tool` / `assess_cursor_tool`
(`crates/sure-core/src/hook_protection.rs`) — the functions the Claude Code and
Cursor integrations call — driven in-process by
`crates/sure-core/tests/adversarial_fixture_detection.rs`.

## The trap this fixture exists to catch

The manifest row is *force push is treated as dangerous in configured protection
mode*, and the trap is that the second half of that sentence is easy to satisfy
without the first:

1. **A product with the danger vocabulary deleted passes it.** Every arbitrary
   command is held for consent by the execution engine, and `base_decision`
   blocks that with a sentence naming no danger. `git push --force` and
   `git status` are `block` alike on such a build, and a test asserting
   `decision == block` cannot tell them apart.
2. **A product that names every push a force push passes it.** `git push origin
   main` destroys nothing, and a classifier that called it a force push would
   satisfy every positive run here while telling users their ordinary pushes
   rewrite published history.
3. **A product that blocks everything forever passes it** — which is, in this
   build, not a hypothetical but the shape of the command route itself.

So the grading test asserts, one run at a time and in this order: **the danger's
wire name** (`force_push`), then **the sentence**, held against the product's own
`danger_reason` return value, and only then the decision kind.

## The seven runs

| Run | Request | Settings | Answer |
| --- | --- | --- | --- |
| `force_push_named_by_the_command` | `Bash` — `git push --force` | `runner`, standard | `block`, `force_push` |
| `the_same_line_in_cursor_s_vocabulary` | `Shell` — `git push --force` (Cursor) | `runner`, strict | `block`, `force_push` |
| `the_short_flag_is_the_same_act` | `Bash` — `git push -f origin main` | `runner`, standard | `block`, `force_push` |
| **`control`** | `Read` — `src/lib.rs` | `runner`, standard | **`allow`**, no danger |
| `an_unforced_push_is_held_for_consent` | `Bash` — `git push origin main` | `runner`, standard | `block`, no danger |
| `a_stacked_line_is_not_read` | `Bash` — `git push --force; echo done` | `runner`, standard | `block`, no danger |
| `a_shell_event_with_no_command_line` | `Bash` — no command at all | `runner`, standard | `block`, no danger |

**`control` is the acceptance.** The held runs are satisfied by a product that
blocks every request, and the control is the one answer that separates that
product from this one: under exactly these settings, an ordinary read of a
project file proceeds.

## The control moves the request, and that is a limit of the product

Every other fixture in this corpus moves one field. This one cannot, and the
declaration says so instead of weakening the assertion to *not blocked*:

> **No command-bearing request reaches `Allowed` in this build.**
> `sure_domain::execution::decide` answers `NeedsConsent` for every
> `ActionKind::ArbitraryCommand` — force flag or not, dangerous or not — and a
> harness hook cannot obtain consent, so `assess_request` blocks it. Under every
> execution mode, every permission set and every protection mode.

So the smallest move available is *the request*: a shell command out, an ordinary
path read in, with the harness, the execution mode, the permissions and the
protection mode all identical. The grading test compares the two declared runs
key by key and requires the keys that differ to be exactly the three the
declaration names. `an_unforced_push_is_held_for_consent` is the measurement of
the limit itself, and it is declared as what it is — held, and no danger named —
rather than left out.

If the product ever lets a command-bearing request through under some setting,
that run goes red, and the fixture is where the new answer will have to be
written down.

## The route, and why there is only one

A force push is read from a **command line** and from nowhere else:

- `claimed_danger` reads a command only for `ActionKind::ArbitraryCommand`;
- `command_danger` needs `safety::read_simple_command` to succeed (any shell
  syntax — a quote, a `;`, a pipe — and it does not) and `safety::classify` to
  answer with a `Source::Rule` row in its `Destructive` class;
- the `push` row reaches `Destructive` through a force flag and through no other
  form, which is why the flag is the danger and the destination (`origin main`)
  is not read at all.

Two consequences are declared in the fixture rather than described here: a
stacked line is held for consent with no danger named, and a shell event carrying
no command line is held the same way. **There is no path route**: a harness event
that names a path and no command can never be named a force push, so on such an
event SURE holds the request and says nothing about what it would do. That is the
limit the manifest row's *where harness exposes pre-action event* is about.

## The allowance half

A hold is worth something only if a user can act on it: `sure hook allow-once`
records a grant, and the next matching request spends it. What a grant can cover
is read off `acts_a_tool_could_be_held_for` / `acts_a_request_could_be_held_for`
(`hook_protection.rs`), and this fixture declares what it expects for `Bash`
under its own settings — both acts a command line can be held for, because the
tool names a shell and the subject is not read until a request arrives.

The store-backed half — held, allowance, one allow, held again — is **not graded
here**. It is graded once, with a real store, in
`crates/sure-cli/src/hook.rs::a_force_push_is_held_and_an_allowance_for_it_lets_one_through`,
and this fixture names that test instead of driving a second copy of it. The name
is a checked pointer: the grading test fails if the file or the function is
renamed.

## How to see it for yourself

```
cargo test -p sure-core --test adversarial_fixture_detection -- dangerous_action
```

The test reads `scenario.json`, checks that the declared settings name every
permission, drives each run through the product, and compares every declared
answer with the product's. It writes nothing anywhere: there is no scratch
directory and no store.

## Watch out

The declaration carries **no** `entry_points` key, and that is load-bearing
rather than an omission. `crates/sure-core/tests/finding_severity_rule.rs`
decides which release-blocking cases have a fixture app by looking for that key,
and adding it here would pull this case into a list it does not belong on.

If a later build reads destinations as well as flags, the run that will disagree
is `the_short_flag_is_the_same_act`; if one starts answering `allow` to a shell
request, it will be the control's field-set assertion or
`an_unforced_push_is_held_for_consent`.

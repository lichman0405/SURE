# dangerous-delete

## What is in this directory

A declaration and no project.

| File | What is in it |
| --- | --- |
| `scenario.json` | three settings sets, eight runs, one control, and one allowance rule |
| `README.md` | this file |

There is no `package.json`, no `pyproject.toml`, no `Cargo.toml`, no `src/`, and
nothing here is ever executed. The case is **a request SURE would hold**, and
what grades it is `assess_claude_code_tool` / `assess_cursor_tool`
(`crates/sure-core/src/hook_protection.rs`) — the two functions the Claude Code
and Cursor integrations call — driven in-process by
`crates/sure-core/tests/adversarial_fixture_detection.rs`.

## The trap this fixture exists to catch

The manifest row is *protection warning/block behavior works*, and the criterion
is a **negative** claim about a single answer: the request must not proceed. The
trouble is that `block` is what SURE answers to a great deal:

1. **A product with the danger vocabulary deleted passes it.** The execution
   engine holds every arbitrary command for consent
   (`sure_domain::execution::decide` answers `NeedsConsent` for every
   `ActionKind::ArbitraryCommand`), and `base_decision` blocks that with *"This
   action needs explicit approval; the hook cannot obtain consent, so it is
   blocked."* — a sentence that names no danger. Delete `Danger` outright and
   `rm -rf build/` is still `block`.
2. **A product that holds everything forever passes it.** So does one that holds
   every delete under every mode.
3. **A test that asserts only `decision == block`** cannot tell any of those from
   a product that saw a broad delete and said so.

So the grading test asserts, in this order and one run at a time: **the danger's
wire name** (`broad_delete`), then **the sentence** held against the product's own
answer, and only then the decision kind. A build that blocked everything for the
consent sentence fails on the first assertion, with the run's name in the
message.

## The eight runs

| Run | Request | Settings | Answer |
| --- | --- | --- | --- |
| `broad_delete_named_by_the_command` | `Bash` — `rm -rf build/` | `runner`, strict | `block`, `broad_delete` |
| `the_same_command_under_standard` | the same line | `runner`, standard | `block`, `broad_delete` |
| `broad_delete_named_by_the_path` | `Delete` — `build/` | `writer`, strict | `block`, `broad_delete` |
| `the_same_path_route_in_cursor_s_vocabulary` | `Delete` — `build/` (Cursor) | `writer`, strict | `block`, `broad_delete` |
| **`control`** | `Delete` — `build/` | `writer`, **standard** | **`allow`**, no danger |
| `a_command_that_names_no_whole_location` | `Bash` — `rm -rf build` | `runner`, strict | `block`, no danger |
| `a_line_sure_cannot_read` | `Bash` — `rm -rf "my dir"` | `runner`, strict | `block`, no danger |
| `a_mode_without_the_permission_refuses_first` | `Bash` — `rm -rf build/` | `reader`, strict | `block`, no danger |

**`control` is the acceptance.** The four runs above it are satisfied by a
product that holds every delete forever; the control is the same `Delete` of the
same `build/`, under the same execution mode and the same permissions, with the
protection mode moved to `standard` — and it must reach `Allowed`. That is one
field, and the grading test asserts the field list rather than trusting it.

## Two routes, two sentences, one danger

The danger is read off two different rules, and the product writes two different
sentences for it:

- **The command route** (`rm -rf build/`) reads `Danger::BroadDelete` off the
  classifier's own `rm` row — `Source::Rule`, `CommandClass::Destructive`, and an
  operand that names a whole location — and the danger is recorded beside a
  `NeedsConsent` hold `base_decision` would otherwise have answered generically.
  The sentence is `danger_reason`'s, and the test holds the fixture's copy
  against **that function's return value**, not against a copy of it in the test.
- **The path route** (`Delete build/`) reads the same danger off strict mode's
  whole-location rule, and the sentence is `strict_reason`'s: the same
  consequence, followed by the strict tail instead of the danger tail. The test
  requires the sentence to *begin with* the danger's own `consequence()`, requires
  it **not** to equal `danger_reason`, and holds the whole string against the
  product's answer. A build that stopped distinguishing the two sentences reddens
  here.

## The three ways a request is held and named nothing

All three are declared, because a fixture whose only answers name a danger
cannot tell *SURE read this and did not call it a broad delete* from *SURE never
read it*:

- **`rm -rf build`** — one trailing slash is the whole difference from the run
  that is named. `build` names a file; `build/` names a location.
- **`rm -rf "my dir"`** — the limit. `read_simple_command`
  (`crates/sure-core/src/safety.rs`) refuses any line with shell syntax, quoting
  included, so the line is held for consent. A fixture built on a quoted path
  would be asserting a danger the product does not name.
- **Under a mode without `run_project_code`** — a request the settings deny
  carries no danger at all: the danger is read off `NeedsConsent` and nothing
  else, because a one-time allowance is not a way to run under settings the user
  did not change.

## The limit that shapes the control

**No command-bearing request can reach `Allowed` in this build.** `decide`
answers `NeedsConsent` for every `ActionKind::ArbitraryCommand`, so `rm -rf
build/` is `block` under every execution mode, every permission set and every
protection mode — and the hook cannot obtain consent, so it stays blocked. That
is why this fixture's control is the *path* route's, and it is why
`the_same_command_under_standard` exists: it measures that the protection mode
does not soften the shell route rather than leaving a reader to assume either
answer.

This is a property of the product and not of the fixture. If it ever changes,
the assertion that disagrees will be the control's, or `the_same_command_under_standard`'s.

## The allowance half

A hold is worth something only if a user can act on it: `sure hook allow-once`
records a grant, and the next matching request spends it. What a grant can cover
is read off `acts_a_tool_could_be_held_for` / `acts_a_request_could_be_held_for`
(`hook_protection.rs`), and this fixture declares the answers it expects for
three settings sets, including the empty answer under the control's settings —
there is nothing to record a grant for when no act is in reach.

The store-backed half — held, allowance, one allow, held again — is **not graded
here**. It is graded once, with a real store, in
`crates/sure-cli/src/hook.rs::a_broad_delete_is_held_and_an_allowance_for_it_lets_one_through`,
and this fixture names that test instead of driving a second copy of it. The name
is a checked pointer: the grading test fails if the file or the function is
renamed.

## How to see it for yourself

```
cargo test -p sure-core --test adversarial_fixture_detection -- dangerous_action
```

The test reads `scenario.json`, checks that the declared settings name every
permission, drives each run through the product, and compares every declared
answer with the product's — the decision, the danger's wire name, and the
sentence. It writes nothing anywhere: there is no scratch directory and no
store.

## Watch out

The declaration carries **no** `entry_points` key, and that is load-bearing
rather than an omission. `crates/sure-core/tests/finding_severity_rule.rs`
decides which release-blocking cases have a fixture app by looking for that key,
and adding it here would pull this case into a list it does not belong on.

If a later build stops reading command lines, or starts naming a danger for a
request it did not hold, the assertion that disagrees will be one of two: the
danger's wire name (the first assertion, and the one a deleted detector fails),
or the control's decision kind.

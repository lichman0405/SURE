# sensitive-read

## What is in this directory

A declaration and no project.

| File | What is in it |
| --- | --- |
| `scenario.json` | three settings sets, seven runs, one control, and three allowance rows |
| `README.md` | this file |

There is no `package.json`, no `pyproject.toml`, no `Cargo.toml`, no `src/`, and
nothing here is ever executed. The case is **a request SURE would hold**, and what
grades it is `assess_claude_code_tool` / `assess_cursor_tool`
(`crates/sure-core/src/hook_protection.rs`) — the functions the Claude Code and
Cursor integrations call — driven in-process by
`crates/sure-core/tests/adversarial_fixture_detection.rs`.

## What this fixture claims, and what it does not

The manifest row is *sensitive-key read can be protected where harness exposes
pre-action event*, and both halves of that sentence are limits rather than
boasts. Stated plainly:

- **SURE protects this read only under `strict`.** Under the default mode
  (`standard`) the same read of the same file is `allowed`. That is not an
  inference: it is the control, and it is graded.
- **Only where the harness hands SURE a pre-action event that names the path.**
  The rule is a path rule. A shell command that reads the same file carries a
  command line and no path, so it is held for consent and nothing is named.
- **Only a read.** A *change* to the same file is held by the same strict rule
  and names no danger at all, so no one-time allowance can cover it.

## The trap this fixture exists to catch

*Held* is satisfiable with no detector at all: strict mode holds this path under
a sentence of its own, and `base_decision` blocks any request that needs consent
with a sentence naming no danger. Delete `Danger` outright and every run in this
directory still answers `block`. So the grading test asserts, one run at a time
and in this order: **the danger's wire name** (`sensitive_read`), then **the
sentence** — which for the strict route must begin with the danger's own
`consequence()` and must not equal `danger_reason` — and only then the decision
kind.

The second trap is the opposite one: a product that held every read would satisfy
every held run here, and `an_ordinary_source_read_is_allowed_under_strict` is
declared so that *strict holds reads* is refuted inside this directory.

## The seven runs

| Run | Request | Settings | Answer |
| --- | --- | --- | --- |
| `credential_area_read_under_strict` | `Read` — `.env` | `reader`, strict | `block`, `sensitive_read` |
| **`control`** | `Read` — `.env` | `reader`, **standard** | **`allow`**, no danger |
| `a_key_file_in_a_named_directory` | `Read` — `secrets/prod.yaml` | `reader`, strict | `block`, `sensitive_read` |
| `the_same_read_in_cursor_s_vocabulary` | `Read` — `.env` (Cursor) | `reader`, strict | `block`, `sensitive_read` |
| `an_ordinary_source_read_is_allowed_under_strict` | `Read` — `src/lib.rs` | `reader`, strict | `allow`, no danger |
| `a_command_line_hides_the_path` | `Bash` — `cat .env` | `runner`, strict | `block`, no danger |
| `a_change_to_the_secret_area_is_held_without_a_danger` | `Write` — `.env` | `writer`, strict | `block`, no danger |

**`control` is the acceptance, and it is the mode pair.** The first run and the
control are the same read of the same file, under the same execution mode and the
same permissions: one field moved, and it is the protection mode. The pair is the
manifest row's *where*, stated as two measurements rather than as prose — and the
control is the only run in this directory that separates *SURE named this read*
from *SURE blocks everything*.

## The sentence an allow carries

Both allows carry a sentence, and neither is an omission:

- standard's says only that nothing in the settings stopped the request and that
  this mode asked no further question. It deliberately does **not** say the read
  is safe — this is the read the two modes differ about, and a sentence implying
  a check that did not happen would be a false green.
- strict's names the five categories it looked for and did not find. That is the
  useful half of an allow: a user learns what the mode is for.

Neither sentence is reachable as a constant, so the grading test holds the
fixture's copy against the product's answer for the run. A rewrite of either
sentence in the product reddens this directory.

## The boundary of the allowance

A hold is worth something only if a user can act on it: `sure hook allow-once`
records a grant, and the next matching request spends it. What a grant can cover
is read off `acts_a_tool_could_be_held_for` / `acts_a_request_could_be_held_for`
(`hook_protection.rs`), and this fixture declares three answers, one of which is
the reason a limit is a limit:

- for `Read` under strict: `sensitive_read`, and that is the whole list;
- for `Read` under standard: **empty** — the control's settings leave no act in
  reach, so there is nothing SURE should offer to record;
- for `Write` under strict: `broad_delete` — a grant for `Write` covers a
  whole-location change and **not** the secret-area hold this fixture declares
  for that mode, because `danger_of` answers `SensitiveRead` for the read pair
  and for nothing else.

The store-backed half — held, allowance, one allow, held again — is **not graded
here**. It is graded once, with a real store, in
`crates/sure-cli/src/hook.rs::a_read_of_credentials_is_held_under_strict_and_an_allowance_for_it_lets_one_through`,
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

If a later build starts holding this read under `standard` too, the control goes
red — which is the point of matching the control to the manifest row's own
wording. If a later build starts naming a danger for a *change* to the secret
area, the run that will disagree is
`a_change_to_the_secret_area_is_held_without_a_danger`, and the allowance row
beside it is what says why that would be a wider change than it looks.

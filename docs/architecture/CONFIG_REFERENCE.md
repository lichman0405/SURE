# `sure.yaml` reference

Every setting SURE reads from a project, what it defaults to, and what the
project is allowed to ask for with it.

`sure.example.yaml` in the repository root is the same surface written as a file
to copy. A test loads that file and asserts it equals the defaults, so the
example cannot drift away from this document's subject matter.

## Where the file is read from

| Name | Read? |
| --- | --- |
| `sure.yaml` in the project root | yes |
| `sure.yml` | **no** — a near-miss name is an error, not a fallback |
| anything else | no |

A missing `sure.yaml` is not an error. It means the project has declared
nothing, and every setting below takes its default. The loader records which of
the two happened, so a report can say whether a setting was chosen by the user
or by the project.

The same file format is read at one other path: the user's own configuration,
outside any project, at `Paths::user_config_file()`. It is the same reader
rather than a second one, so a near-miss `sure.yml` beside it is the same
mistake. `Config::load_file` reads a file at exactly the path it is given;
`Config::load` reads `sure.yaml` from a project root. Which layer a setting came
from is `docs/architecture/CONFIG_AUTHORITY.md`.

## Rules that apply to every setting

1. **An unknown setting stops the run.** SURE does not warn and continue. A
   setting that was silently dropped looks exactly like a setting that was
   applied, so the difference has to be visible.
2. **A setting written twice stops the run.** There is no last-one-wins
   resolution. Whichever value were picked, the other would vanish without a
   word, and a setting written twice is usually one being changed.
3. **A value outside the accepted list stops the run**, and the message names
   the accepted values plus the closest one when the mistake looks like a typo.
4. **A credential-shaped key stops the run**, and its value is never printed.
5. **A combination that cannot take effect stops the run.** See
   [Contradictions](#contradictions).
6. **A path that leaves the project stops the run.**

A UTF-8 byte-order mark is tolerated, and CRLF line endings are normal input —
both are what Windows editors write by default.

## Settings

### `privacy`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `mode` | enum | `local_first` | `local_first`, `fully_local` (see below) |
| `full_recording` | boolean | `false` | `true`, `false` |
| `telemetry` | boolean | `false` | `true`, `false` |

`local_first` keeps evidence on this machine and allows external analysis only
where it is separately configured. `fully_local` sends nothing out at all, and
cannot be combined with an external analysis provider.

`cloud_enhanced` is described in the privacy documentation as a future mode and
is **refused** rather than accepted. Nothing in this release implements it, and
accepting the setting would let a project file claim a privacy arrangement SURE
does not provide.

`full_recording` and `telemetry` are opt-in. Both are reported as requests, and
neither takes effect without higher-authority approval.

### `protection`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `mode` | enum | `standard` | `standard`, `strict` |

`custom` needs a rule editor and a rule format, neither of which exists in this
release, and is refused for the same reason as `cloud_enhanced`.

### `execution`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `mode` | enum | `inspect_only` | `inspect_only`, `host_confirmed` |
| `allow_dependency_install` | boolean | `false` | `true`, `false` |
| `allow_network` | boolean | `false` | `true`, `false` |

`inspect_only` runs none of the project's code. `host_confirmed` allows the
project's code to run on this machine, and always needs confirmation for an
action SURE cannot classify.

Both booleans are **requests for a permission**, never a grant. Setting
`mode: host_confirmed` is a request to run project code; it does not grant it.

### `analysis`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `provider` | enum | `disabled` | `disabled`, `local_command`, `claude_cli`, `openai_compatible` |
| `endpoint` | text or `null` | `null` | an absolute `http` or `https` URL, with no password in it |
| `model` | text or `null` | `null` | any name the provider recognises |

`disabled` consults no model at all. `local_command` runs a program on this
machine and nothing leaves it; the other two are **external** and are reported
as such — `openai_compatible` because a service receives the content, and
`claude_cli` because the program it runs talks to one.

Both `endpoint` and `model` are only meaningful with a provider that uses a
model. Setting either while the provider is `disabled` is a contradiction and
is refused, because the setting could never take effect and leaving it in place
would suggest analysis is configured when it is not.

A password inside `endpoint` is refused, and the value is never printed. Keep
credentials in user-level configuration or an environment variable.

### `project_intent`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `goal` | text or `null` | `null` | a non-empty goal |
| `spec_path` | text or `null` | `null` | a path inside this project |

Both are read from a project-controlled file, so both are classified as
`IntentSource::ProjectSpec`, which is **documentation, not a user requirement**.
A goal written here can never become the requirement the project is graded
against, because a file anyone can edit cannot state what the user asked for.
Without a requirement from a trusted source, every report states the
after-the-fact limitation instead of claiming the work is complete.

An empty `goal` is refused rather than treated as absent.

### `checks`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `existing_tests` | boolean | `true` | `true`, `false` |
| `start_local_services` | enum | `auto` | `auto`, `always`, `never` |
| `browser_probe` | enum | `auto` | `auto`, `always`, `never` |

`always` and `never` are requests as well as settings: turning a check off
reduces what SURE covers, and a reduced scope is reported rather than passed
over.

> YAML 1.1 read `yes`, `no`, `on` and `off` as booleans. YAML 1.2 — the version
> this file is parsed as — does not, so `existing_tests: yes` is a string that
> is neither `true` nor `false`. SURE reports the two values that work rather
> than leaving the user to work out why the type did not match.

### `report`

| Key | Type | Default | Accepted values |
| --- | --- | --- | --- |
| `format` | enum | `human` | `human`, `json` |

## What a project may and may not ask for

A project file is untrusted input: the project being checked may be controlled
by the same AI whose work SURE is evaluating. `docs/architecture/CONFIG_AUTHORITY.md`
sets the authority order, and this file can only ever sit below the user's own
configuration.

**Requests** — parsed, reported by `Config::requested_privileges`, and inert
until a higher authority agrees. The user's own configuration at
`Paths::user_config_file()` is that higher authority; a request only this file
makes is refused and recorded as a refusal, not granted and not dropped. What
each request becomes is in `docs/architecture/CONFIG_AUTHORITY.md`:

| Setting | Request |
| --- | --- |
| `execution.mode` other than `inspect_only` | run project code |
| `execution.allow_dependency_install: true` | install dependencies |
| `execution.allow_network: true` | network access |
| `privacy.full_recording: true` | full recording |
| `privacy.telemetry: true` | telemetry |
| `analysis.provider` that is external | connect to a service |

**Scope reductions** — reported by `Config::scope_reductions`, so that a check
that did not happen is never mistaken for a check that passed:

| Setting | Reduction |
| --- | --- |
| `checks.existing_tests: false` | the project's own tests were not run |
| `checks.start_local_services: never` | local services were not started |
| `checks.browser_probe: never` | the interface was not checked in a browser |

## Contradictions

Each of these is refused, naming both settings and saying why they cannot both
hold:

| Combination | Why |
| --- | --- |
| `privacy.mode: fully_local` with an external `analysis.provider` | fully-local means nothing leaves the machine; both named external providers send what they are given to a service |
| `analysis.provider: disabled` with `endpoint` or `model` set | no model is consulted, so the setting could never take effect |
| `execution.mode: inspect_only` with `allow_network` or `allow_dependency_install` | nothing runs, so nothing could use the permission; it would read as an allowance SURE does not have |

## Connection to the frozen semantics

This document describes a *request*. The rules about what those requests become
— execution modes, permissions, grantors, intent trust — are frozen in
`docs/architecture/FROZEN_SEMANTICS.md` and enforced in `crates/sure-domain`.
Nothing here restates them; this file can only narrow them or ask for them.

## Enforced by

`crates/sure-core/src/config/`:

| File | Holds |
| --- | --- |
| `mod.rs` | the model, `from_yaml`, `load`, `load_file`, validation, the request and reduction lists |
| `values.rs` | the seven enums, their wire names and what each one means |
| `error.rs` | every message a bad file can produce |
| `authority.rs` | which layer a setting came from, and what a lower layer may do with it |

`crates/sure-core/src/redact.rs` is the second line of defence against a secret
reaching a message. It lives at the crate root rather than here because
`diagnostics` applies it to every recorded field as well; see
`docs/architecture/DIAGNOSTICS.md`.

`crates/sure-core/tests/config_loading.rs` covers the filesystem path — spaces
and non-ASCII directory names, byte-order marks, CRLF, a non-UTF-8 file, the
`yaml`/`yml` near-miss, an absent file, and a directory occupying the file's
name.

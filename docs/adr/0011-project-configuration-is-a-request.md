# ADR 0011 — Project configuration is a request, not a grant

Status: Accepted

Date: 2026-09-14

## Context

`docs/architecture/CONFIG_AUTHORITY.md` states the authority order and the rule
that a lower-authority source cannot weaken a higher-authority restriction. The
hardest case in that document is `sure.yaml`: an ordinary-looking settings file
at the root of the project being checked.

That project may be controlled by the same AI whose work SURE is evaluating.
The file is therefore attacker-controlled input in the threat model, not
configuration in the usual sense — while also being the natural place for a
person to write "this project runs its test suite with `pnpm test`".

The failure mode to avoid is specific and quiet. A model that reads `sure.yaml`
into a struct with the fields `execution_mode`, `allow_network` and
`allow_dependency_install` has already granted them: by the time anything else
looks at the struct, "the project asked for network access" and "SURE has
network access" are the same value. Nothing in the code says a decision was
made. The AI under evaluation can then widen its own permissions by writing a
file, and every subsequent report will describe the result as engineered
behaviour rather than an escalation.

This is the false green `CLAUDE.md` ranks above a visible error: nothing fails,
nothing warns, and the report is wrong in the direction that flatters the agent.

## Decision

**Parse project configuration into a request, and require a separate authority
step to turn any of it into a capability.**

Concretely:

1. `sure-core::config::Config` is a faithful description of what the file says.
   Reading it grants nothing. No code path takes effect from a `Config` alone.
2. Every setting that would widen what SURE may do is enumerated by
   `Config::requested_privileges`, which returns `sure_domain`'s existing
   `ProjectRequest` vocabulary rather than a new one. A test walks every
   privileged flag and asserts the list is complete, so a field added later
   cannot be forgotten.
3. Every setting that narrows coverage is enumerated by
   `Config::scope_reductions`, so a check that did not happen is never
   mistaken for a check that passed.
4. `ProjectRequest::permission()` is the only mapping from a request to a
   permission, and `ProjectRequestEscalated` — which `can_grant()` rejects — is
   how a project-originated request is recorded.
5. A project-file goal is classified `IntentSource::ProjectSpec`, and
   `Config::goal_source()` is a `const` returning that value. A goal read from
   a project file can never become a user requirement, with a test asserting
   `!is_user_requirement()`.
6. An invalid file **stops the run**. There is no fallback to defaults. A file
   that could not be understood is not a file that was absent, and running with
   defaults while the user believes their settings are in force is the same
   class of silent wrongness.
7. An unknown setting stops the run rather than being ignored. A setting that
   was dropped looks exactly like a setting that was applied.
8. The model has **no field that accepts a credential**, a credential-shaped
   key stops the read, and a password inside an otherwise legal value fails
   validation. Redaction (`config::redact`) is the second line of defence, not
   the first.

## Alternatives rejected

**A permissive model with a separate "is this allowed" check.** Two places to
be right, and the second is the one nobody updates. Keeping the request inert
in the type makes the safe path the only one that compiles.

**Restricting what a project may write at all.** Too blunt: a project genuinely
is the authority on which checks apply to it, and refusing to let it describe
itself would push users into user-level configuration for per-project facts,
which is worse to maintain and hides the same information.

**Treating an invalid file as empty with a warning.** A warning in a settings
path is a message users learn to scroll past. Since the entire point is that a
dropped setting is indistinguishable from an applied one, only stopping is
honest.

**Merging project settings into user settings field by field, taking the
stricter value.** Sounds conservative, but it silently reinterprets intent: a
user who did not mention a setting has not expressed a preference to be
maximised, and a merged config cannot be reported back to them in terms of the
file they wrote.

## Consequences

- The authority layer (P1-T011) has a complete, tested input. It cannot miss a
  privileged setting, because the list is derived from the same struct and
  checked against every flag in it.
- Reports can distinguish "the project turned this off" from "you turned this
  off", because the loader records where the settings came from.
- Project configuration is more annoying to write than it would be otherwise:
  a typo stops the run instead of being ignored, and a setting that cannot take
  effect is an error rather than a no-op. This is the intended cost.
- Two documented modes (`cloud_enhanced`, `custom` protection) are parsed and
  refused rather than accepted. Accepting a setting this release does not
  implement would let a file claim a privacy or protection arrangement SURE
  does not provide.
- `settings` with no file present is distinguishable from settings that failed
  to load, so a report never has to guess which happened.

## Frozen semantics

Requests, permissions and intent trust: `docs/architecture/FROZEN_SEMANTICS.md`.
Enforced by `crates/sure-core/src/config/mod.rs` and
`crates/sure-domain/src/execution.rs`. The settings surface itself is described
in `docs/architecture/CONFIG_REFERENCE.md`.

## Revisit when

A second party — organization policy, or another tool — needs to write project
configuration on the project's behalf. That introduces an authority that is
neither the user nor the project, and the request/grant split above assumes
there are only two.

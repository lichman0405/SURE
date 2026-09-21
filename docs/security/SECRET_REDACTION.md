# Secret redaction

Redaction must occur before user-visible reports and preferably before durable recording/model context.

Sources:
- explicitly configured secret names/values;
- environment-variable names likely to contain secrets;
- common token/key formats;
- provider credentials.

Requirements:
- avoid logging raw environment values;
- redact stdout/stderr before persistent full-recording write where feasible;
- test false positives as well as known secret patterns;
- never claim perfect secret detection.

## Where this release stands

The list above is the **requirement**. What runs is narrower, and the sources
that are not wired are named here rather than left for a reader to discover:

- **Built in, and always on.** The engine in `sure_core::redact` is the one that
  runs and the only one. It replaces a password inside a URL authority, the value
  assigned to a credential-shaped name (`api_key=...`), PEM **private key**
  blocks (certificates are left readable), bearer tokens, and runs of characters
  matching a known token shape — among them `sk-`, `ghp_`, `github_pat_`,
  `xox[bpars]-`, `ya29.`, `AKIA`/`ASIA` and a JSON web token's `eyJ`.
- **Configured literals and patterns are not wired in.** `redaction.*` can be
  written to a settings file, but nothing in this release builds a redactor from
  one. `sure settings` says so in its own words: *"the redaction that runs is the
  built-in one. Nothing in this release builds a redactor from a settings file,
  so these rules would change nothing."* A user who names their project's
  internal token there gets no protection from it, which is why the answer is
  printed rather than left to be assumed.
- **There is no environment-variable name list.** Nothing enumerates variable
  names, and SURE does not read the environment for secrets at all. The
  name-based rule runs where a name is a name: one of SURE's own diagnostics,
  where a credential-shaped key means the value is not recorded at all; its own
  settings check, which refuses a credential-shaped setting by name; and, inside
  a run of free text, a credential-shaped name written to the left of an `=`.
  It is **not** consulted for the *keys* of a record — `redact_value` redacts the
  string leaves of a document and leaves its keys alone, because a rewritten key
  would fail a schema check and read as a malformed document. The consequence is
  that what protects a record is the pattern list: a secret under a
  credential-named key, in a shape none of the passes knows, is stored as it was
  received. What contains that structurally is the config model having no
  credential field — which is the code's own reason for the gap, and not a claim
  that the pattern list is enough.

Two further things a reader should not have to infer:

- **Redaction runs on the way into the store, not on every surface.** Every
  document is redacted before it is validated and written, and the
  full-recording payload is redacted before the write, so a raw transcript is
  the redacted one. A prompt built for a provider is redacted where it is built,
  on a line this build never reaches because no check asks for model-backed
  analysis. Reports are a different path: they escape control characters and run
  no credential engine, so what a report shows is either a value read back from
  the store — redacted on the way in — or a value SURE read for itself.
- **The rules have one known hole, and the code states it.** A credential passed
  as a flag's *separate argument* — `--api-key abcdef1234567890` — is not caught.
  `redact.rs`'s own test asserts that and gives the reason: treating `name value`
  as an assignment would redact the word "is" out of `--api-key is required`. A
  limitation stated without its consequence is decoration, so: this is what
  "never claim perfect secret detection" means here, and why the requirement is
  the last line of the list above rather than a footnote to it.

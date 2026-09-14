# Storage and data paths

## Project-local

`.sure/` may hold ephemeral project/run caches and is gitignored by default.

A user may commit `sure.yaml` intentionally.

## User-level durable data

Use OS-appropriate application-data locations through a well-tested path abstraction rather than hard-coded `~/.sure` everywhere.

Windows must use the chosen Rust path library's per-user application-data/config conventions (for example LocalAppData/RoamingAppData as appropriate), not hard-coded home-directory paths. macOS/Linux use their native equivalents through the same abstraction.

Expose:

```text
sure doctor
sure history
sure history delete ...
sure config paths
```

so users can see/delete what is stored.

## Storage rules

- schema migrations are versioned;
- do not hold synchronous DB handles across async `.await`;
- event writes are transactional/robust against short-lived concurrent hook processes;
- full recordings are clearly distinguishable and deletable;
- secrets are redacted before durable write where possible.

## Authoritative evidence boundary

The checked project is not a trusted place to store authoritative evidence. An agent working in the repository may edit `.sure/`.

Therefore durable evidence/history used for verdict/claim truth must live in user-level application data outside the working tree. Project `.sure/` may contain cache IDs, local non-authoritative artifacts or links, but modifying it must not let project code fabricate a passing authoritative history.

Cryptographic tamper-proof attestation is not a v0.1 claim.

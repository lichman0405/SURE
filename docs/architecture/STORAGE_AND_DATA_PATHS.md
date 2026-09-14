# Storage and data paths

## Project-local

`.sure/` may hold ephemeral project/run caches and is gitignored by default.

A user may commit `sure.yaml` intentionally.

## User-level durable data

Use OS-appropriate application-data locations through a well-tested path abstraction rather than hard-coded `~/.sure` everywhere.

Windows must use the chosen Rust path library's per-user application-data/config conventions (for example LocalAppData/RoamingAppData as appropriate), not hard-coded home-directory paths. macOS/Linux use their native equivalents through the same abstraction.

### Resolved locations

The abstraction is `sure_core::paths`. It asks the platform, through the `dirs`
crate, rather than reading environment variables directly: the correct Windows
API is `SHGetKnownFolderPath`, this workspace forbids `unsafe`, and a hand-rolled
read of `%LOCALAPPDATA%` silently produces a *relative* path when the variable is
unset — which resolves against the current directory, which is the project being
checked.

| What | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Data (`Paths::data_dir`) | `%LOCALAPPDATA%\SURE` | `~/Library/Application Support/SURE` | `~/.local/share/SURE` |
| Config (`Paths::config_dir`) | `%APPDATA%\SURE` | `~/Library/Application Support/SURE` | `~/.config/SURE` |
| Project cache | `<project>\.sure` | `<project>/.sure` | `<project>/.sure` |

Data is the machine-local location and config is the roaming one, deliberately:
history is not something a roaming profile should carry between machines, while
settings are exactly what roaming is for. On macOS the two coincide, which is
correct there and is not a reason to collapse them on Windows.

There is no separate user-level *cache* root. On Windows `data_local_dir` is
already `%LOCALAPPDATA%`, so a third location would name the same place twice.
What is regenerable is the project cache, and that is `project_cache_dir`, a
separate function from `Paths` so a caller cannot reach for the trusted location
by accident.

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

This is enforced, not merely documented. `Paths::ensure_outside(project_root)`
refuses a project root that contains the data or config directory, and
`Paths::from_roots` refuses a relative location so that a store can never be
resolved against the current directory. The case is not hypothetical: a user who
runs SURE on their home directory has a project root containing
`%LOCALAPPDATA%`, and the correct answer there is to stop rather than write
anyway.

The comparison is `sure_core::paths::compare`, which is component-wise rather
than textual (`C:\project-evil` is not inside `C:\project`), case-folding on the
platforms that case-fold (reporting "within" when a comparison is genuinely
ambiguous, because that is the direction that refuses to write), and treats a
canonicalised `\\?\C:\...` path as the same location as `C:\...`.

Cryptographic tamper-proof attestation is not a v0.1 claim.

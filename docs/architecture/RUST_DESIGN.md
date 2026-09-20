# Rust design

## Baseline

- Rust 1.98.1 stable.
- Edition 2024.
- No nightly requirement.
- Primary local host: native Windows 11 x64, `x86_64-pc-windows-msvc`.
- Required CI/release core targets include Windows x64 MSVC, macOS (Apple Silicon) and Linux x64. The macOS release target is `aarch64-apple-darwin`; no `x86_64-apple-darwin` artifact is produced or claimed — see `docs/development/RELEASE_PROCESS.md`, "What the macOS Intel archive is, and why none exists yet".

## Dependency strategy

Choose current compatible versions during implementation rather than preloading the bootstrap with unnecessary dependencies.

Likely categories:
- Tokio for asynchronous process/network orchestration;
- clap for CLI;
- serde family for versioned protocols/config;
- tracing for structured local diagnostics;
- thiserror and bounded anyhow use at application edges;
- bundled/portable SQLite binding;
- rustls-oriented HTTP stack where practical;
- hashing/ID/path/walk/test utilities.

Keep the dependency tree small enough that native Windows packaging remains tractable.

## Process execution

Centralize:
- executable + argument vector;
- cwd;
- environment allow/deny behavior;
- timeout/cancellation;
- stdout/stderr capture and bounds;
- exit/termination information;
- execution-trust classification;
- timing/evidence metadata.

Avoid constructing shell command strings from untrusted paths/arguments when direct process args work.

Windows process-tree termination must be deliberate and tested; do not pretend Unix signal semantics apply.

## Git

Invoke system Git through one abstraction.

Test:
- dirty state;
- untracked files;
- HEAD/diff/fingerprint;
- spaces and Unicode paths;
- Windows path separators and case-insensitive behavior;
- long-path pressure;
- macOS/Linux behavior through CI.

## Storage

Do not hold synchronous DB resources across `.await`.

All persistent schema changes use explicit migrations.

Short-lived hook processes may contend for the same local database; busy/retry/transaction semantics must be deliberate and tested.

## Safety

Avoid `unsafe` by default. Any unavoidable use requires an ADR, narrow scope and focused tests.

## Time

Inject a clock/test time source where sleeping would make tests slow or flaky.

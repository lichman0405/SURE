# Linux support

Linux is a required Rust-core CI/release target.

The release path for it is `sh scripts/Build-Release.sh --target
x86_64-unknown-linux-gnu`, the same script the two macOS targets use, run by the
`package-linux` job in `.github/workflows/release-dry-run.yml`.
`docs/development/RELEASE_PROCESS.md`'s *"What the Linux x64 archive is,
concretely"* says what that archive is, what has been verified and where, and
what is not yet produced by any run.

v0.1 must avoid Windows-only behavior in portable core logic while still implementing native Windows behavior correctly behind abstractions. Harness integration packages should use portable shell/Rust launchers where practical.

Container execution features will likely be easiest to validate on Linux but must not become required for ordinary static checks.

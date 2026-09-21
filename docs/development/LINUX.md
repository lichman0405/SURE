# Linux support

Linux is a required Rust-core CI/release target.

The release path for it is `sh scripts/Build-Release.sh --target
x86_64-unknown-linux-gnu`, the same script the two macOS targets use, run by the
`package-linux` job in `.github/workflows/release-dry-run.yml`.
`docs/development/RELEASE_PROCESS.md`'s *"What the Linux x64 archive is,
concretely"* says what that archive is, what has been verified and where, and
which run produced it: run `35526723850` built, checksummed and ran
`sure-0.0.0-bootstrap-x86_64-unknown-linux-gnu.tar.gz` — 4599814 bytes, sha256
`44cd8357…bb2a` — so an artifact does exist rather than only the path that makes
one.

Which machines can run it is narrower than "Linux". The shipped binary requires
**glibc 2.39 or newer**, because the job builds on `Ubuntu 24.04.5 LTS` and the
binary carries that image's libc floor. It does **not** run on Ubuntu 22.04
(glibc 2.35) or on Debian 12 (glibc 2.36), and lowering that floor means building
inside an older container, which no step of this repository does today.

v0.1 must avoid Windows-only behavior in portable core logic while still implementing native Windows behavior correctly behind abstractions. Harness integration packages should use portable shell/Rust launchers where practical.

Container execution features will likely be easiest to validate on Linux but must not become required for ordinary static checks.

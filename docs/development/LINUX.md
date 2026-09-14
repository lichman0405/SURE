# Linux support

Linux is a required Rust-core CI/release target.

v0.1 must avoid Windows-only behavior in portable core logic while still implementing native Windows behavior correctly behind abstractions. Harness integration packages should use portable shell/Rust launchers where practical.

Container execution features will likely be easiest to validate on Linux but must not become required for ordinary static checks.

# Supply-chain security

v0.1 engineering should include:

- pinned Rust toolchain;
- committed lockfiles when dependencies are introduced;
- dependency update automation;
- `cargo audit`/equivalent release check;
- license review/deny configuration as dependencies appear;
- Node plugin packages with minimal/no dependencies where practical;
- no curl-pipe-shell in automated SURE project checks;
- release checksums.

Cryptographic artifact attestation/signing is future-capable but not required to prove product value in v0.1.

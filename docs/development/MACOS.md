# macOS support — secondary development/release target

macOS remains a required Rust-core CI and release target, but is not the primary local development environment for v0.1.

Required release targets include **Apple Silicon**, which is built and uploaded as a workflow artifact. Neither macOS archive is published — this repository has no GitHub Releases — so "built" is the whole of what is true of either. The Intel support policy is no longer "to be decided": `P15-T006` added the target and the packaging path, and `.github/workflows/release-dry-run.yml`'s `package-macos-intel` job builds, checksums and runs it on a native Intel runner. Run `35514769749`, job `106088732392`, is that measurement, and the boundary is written down in `docs/development/RELEASE_PROCESS.md`, `### What the macOS Intel archive is, concretely`. In short — an `x86_64-apple-darwin` artifact **is produced and is run**, it is **not signed or notarized**, and it is **not published**.

Intel Macs can still **build SURE from source**: `cargo build` there is a native build, and `scripts/test-sure-environment.sh` grades an Intel host PASS for that reason. That is a statement about development hosts and not about releases.

Core code must avoid Windows-only assumptions. Shell helpers may exist for macOS/Linux, but authoritative bootstrap instructions live in `docs/development/WINDOWS.md`.

macOS-specific concerns include:
- GUI applications may inherit a reduced PATH;
- default filesystem is commonly case-insensitive;
- app-data paths must use native conventions;
- process-tree behavior differs from Windows;
- plugin launchers must not assume Homebrew locations.

macOS signing/notarization is an optional credential-dependent release enhancement, not a v0.1 correctness claim.

**It is also not held, and that is a different sentence.** No Apple Developer ID certificate and no notarization credential are configured for this project, so neither macOS archive is signed, notarized or stapled, and a person who downloads one meets Gatekeeper rather than an Apple-vouched artifact. Nothing in this repository has observed what Gatekeeper does with either archive on a Mac, and nothing in it asks anyone to turn Gatekeeper off. `docs/development/RELEASE_PROCESS.md`, `### macOS: no Developer ID, no notarization and no staple`, is the authoritative statement of what is missing, who would have to provide it and what the consequence is; its own `### What must not be claimed about macOS` lists the phrasings that section forbids.

# macOS support — secondary development/release target

macOS remains a required Rust-core CI and release target, but is not the primary local development environment for v0.1.

Required release targets include **Apple Silicon**, which is built and published. The Intel support policy is no longer "to be decided": `P15-T006` measured what could be measured from a Windows host and wrote the boundary down in `docs/development/RELEASE_PROCESS.md`, `### What the macOS Intel archive is, and why none exists yet`. In short — **no `x86_64-apple-darwin` artifact is produced or claimed**, the check and packaging path for one exists and has been exercised against real bytes, and whether it can be built is the one question `.github/workflows/release-dry-run.yml`'s `package-macos-intel` job would answer. That job has never run.

Intel Macs can still **build SURE from source**: `cargo build` there is a native build, and `scripts/test-sure-environment.sh` grades an Intel host PASS for that reason. That is a statement about development hosts and not about releases.

Core code must avoid Windows-only assumptions. Shell helpers may exist for macOS/Linux, but authoritative bootstrap instructions live in `docs/development/WINDOWS.md`.

macOS-specific concerns include:
- GUI applications may inherit a reduced PATH;
- default filesystem is commonly case-insensitive;
- app-data paths must use native conventions;
- process-tree behavior differs from Windows;
- plugin launchers must not assume Homebrew locations.

macOS signing/notarization is an optional credential-dependent release enhancement, not a v0.1 correctness claim.

# macOS support — secondary development/release target

macOS remains a required Rust-core CI and release target, but is not the primary local development environment for v0.1.

Required release targets should include Apple Silicon where practical and document Intel support policy based on CI/toolchain feasibility.

Core code must avoid Windows-only assumptions. Shell helpers may exist for macOS/Linux, but authoritative bootstrap instructions live in `docs/development/WINDOWS.md`.

macOS-specific concerns include:
- GUI applications may inherit a reduced PATH;
- default filesystem is commonly case-insensitive;
- app-data paths must use native conventions;
- process-tree behavior differs from Windows;
- plugin launchers must not assume Homebrew locations.

macOS signing/notarization is an optional credential-dependent release enhancement, not a v0.1 correctness claim.

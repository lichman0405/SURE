# v0.3.1 hotfix

The v0.3 Windows environment checker contained two PowerShell bugs.

1. Its native command helper declared a parameter named `$Args`. PowerShell has an automatic `$args` variable (case-insensitive), so argument forwarding was broken. Commands such as `git --version` were effectively invoked as bare `git`.
2. It assigned to `$host`, which collides with PowerShell's built-in read-only `$Host` variable.

Both are fixed in v0.3.1.

If you already extracted v0.3, replacing only `scripts/Test-SureEnvironment.ps1` is sufficient for this hotfix. The full v0.3.1 ZIP is provided so a clean re-extract is safer.

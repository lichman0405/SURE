# GitHub workflow

Canonical remote:

```text
https://github.com/lichman0405/SURE.git
```

Default branch: `main`.

Autonomous implementation branch:

```text
claude/v0.1-autonomous
```

## Bootstrap on Windows

Run:

```powershell
.\scripts\Publish-Bootstrap.ps1
```

The script performs the first explicit publish. It aborts rather than overwrite incompatible remote history and never force-pushes.

A POSIX `publish-bootstrap.sh` may remain for secondary macOS/Linux developer use, but PowerShell is canonical.

## Commit policy

Accepted tasks use:

```text
P4-T003: implement Python deterministic checks
```

Do not bundle unrelated tasks merely to reduce commit count.

## Push policy

- no force push;
- phase-boundary checkpoint pushes when authenticated;
- local work continues if GitHub temporarily fails;
- final implementation is presented as a branch/PR for owner review;
- autonomous flow does not merge to `main` without explicit active-session authorization.

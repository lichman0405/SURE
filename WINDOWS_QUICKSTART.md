# Windows quickstart

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\Test-SureEnvironment.ps1
.\scripts\Bootstrap-Sure.ps1
.\scripts\Publish-Bootstrap.ps1   # first publish only
claude
```

Inside Claude Code:

```text
/build-sure
```

Resume later with `/resume-sure`.

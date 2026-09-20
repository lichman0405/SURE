# Installing SURE on Windows

Per-user. No administrator, no elevation prompt, no service, no `PATH` edit, no
registry key, and no driver. Everything below is one PowerShell script that
copies files and one that removes them.

```powershell
& .\scripts\Install-Sure.ps1 -Archive <path>\sure-<version>-x86_64-pc-windows-msvc.zip
& .\scripts\Uninstall-Sure.ps1
```

## Where it installs

```text
%LOCALAPPDATA%\SURE\
    bin\sure.exe            what the install writes
    bin\LICENSE
    bin\RELEASE.txt
    install-manifest.json   what the install wrote, with each file's SHA-256

    sure.db                 the user's evidence, which the install never writes
```

`%LOCALAPPDATA%\SURE\bin\sure.exe` is not a preference chosen here. Three
launchers already in this repository resolve exactly that path, and a different
destination would be a binary those integrations cannot find:

| file | line | what it does |
| --- | --- | --- |
| `integrations/agent-plugin/scripts/install.ps1` | 7 | `Join-Path $env:LOCALAPPDATA 'SURE\bin\sure.exe'`, third in the order `$env:SURE_BIN` → `Get-Command sure` → here |
| `integrations/claude-code/scripts/sure-hook.ps1` | 14 | the same candidate, third in the same order |
| `integrations/claude-code/scripts/sure-mcp.ps1` | 16 | the same path, with `[Environment]::GetFolderPath('LocalApplicationData')` as a fallback when `%LOCALAPPDATA%` is unset |

`Grep` over `integrations/` for `LOCALAPPDATA` returns those three and nothing
else; `crates/sure-cli/tests/install_flow.rs` drives the third of them end to end
against a real install.

The known-folder lookup is not hand-rolled. `crates/sure-core/src/paths/mod.rs`
records why: a bare read of `%LOCALAPPDATA%` "would quietly produce a *relative*
path whenever that variable is unset". Both scripts read the variable first and
ask the platform (`[Environment]::GetFolderPath('LocalApplicationData')`) second,
which is the same two steps `sure-mcp.ps1:16` takes, and a test compares the two
scripts' text so that a fix to one is a fix to both.

## The directory is also the data directory

`crates/sure-core/src/paths/mod.rs` puts the user's durable evidence — the
`sure.db` history a verdict is read from — at `<per-user data>\SURE`, which on
Windows is the same directory the program is installed into. So on a machine
where SURE has been used, the two live side by side. This is the fact that
shapes the uninstaller, and it is why nothing here is described as "the install
directory": it is the user's data directory that happens to hold an install.

## The archive

`scripts/Build-Release.ps1` produces it and `docs/development/RELEASE_PROCESS.md`
specifies it: `sure-<version>-x86_64-pc-windows-msvc.zip`, one top-level
directory of the same name holding `sure.exe`, `LICENSE` and `RELEASE.txt`, with
a `<archive>.sha256` beside it in `sha256sum` format.

The installer **requires** both files. It checks the checksum before it extracts
anything, and there is no switch to skip that check — a check that can be turned
off is not a check. It also refuses a `.sha256` that is missing, empty, has more
than one line, carries a CR, starts with a BOM, or names a different file, and it
refuses an archive whose entries would extract outside the scratch directory.

### Verifying a download

The `.sha256` is an integrity check over the bytes that were shipped: it shows
they arrived unchanged. It is **not** a signature and it does not establish who
built them — see `## Signing` in `RELEASE_PROCESS.md`.

`scripts/Build-Release.ps1 -Phase Verify` does **not** verify a download. It
needs `target/tmp/release-gate.json` in the checkout it is run from, because the
gate path is derived from the script's own location, so a person holding only a
downloaded archive cannot run that phase at all. It also writes `logs\` and
`scratch\` into the directory holding the archive — that is the run's own
evidence and is by design, but it means that directory is not left undisturbed.
Do not use `-Phase Verify` to check a download, and do not describe it as leaving
a directory as it found it.

The installer checks the checksum itself, which is what a person downloading an
archive actually needs.

## The build is unsigned

There is no Authenticode signature. Authenticode credentials are external and
`RELEASE_PROCESS.md` says not to fake signing, so this build has none, and the
installer says so in its output rather than implying a publisher. The first run
of the installed `sure.exe` may show a SmartScreen or "unknown publisher"
warning. That is the expected consequence of an unsigned build, not a fault in
the archive or in the install.

## Options

| switch | default | what it does |
| --- | --- | --- |
| `-Archive` | required | the `.zip` to install |
| `-Checksum` | `<archive>.sha256` | the checksum file to check it against |
| `-InstallRoot` | `%LOCALAPPDATA%\SURE` | where to install; must be absolute |
| `-ScratchDirectory` | a fresh directory under `%TEMP%` | where the archive is unpacked |
| `-Force` | off | replace files at the destinations that this install did not write, naming each one before it does |

`-ScratchDirectory` is used with care: a directory the installer created is
removed whole at the end, but one the caller named is only ever touched at its
`unpacked` child. `-ScratchDirectory C:\` is a value a person can type.

Exit codes: `0` installed, `2` stopped — with the reason printed, and nothing
installed or replaced.

## PATH

**The installer writes no `PATH` entry, machine or user.** Neither does the
uninstaller remove one. This is checked rather than claimed: a test reads the
shipped text of both scripts and fails if any line calls
`SetEnvironmentVariable` outside a line that only prints it.

The program is reachable three ways without any `PATH` entry:

1. by its full path, `%LOCALAPPDATA%\SURE\bin\sure.exe`;
2. by `$env:SURE_BIN`, if you set it;
3. by the three launchers above, which resolve the per-user path directly.

If you want `sure` on `PATH` for your own shells, that is a change to your user
environment and it is yours to make. The installer prints the line and does not
run it:

```powershell
[Environment]::SetEnvironmentVariable(
  'Path',
  ([Environment]::GetEnvironmentVariable('Path', 'User') + ';' + "$env:LOCALAPPDATA\SURE\bin"),
  'User')
```

Open a new shell afterwards. This is per-user, not machine-wide, and needs no
administrator.

### How "documented and tested" is satisfied here

The `PATH` behavior above is not a paragraph standing on its own. What is
measured, in `crates/sure-cli/tests/install_flow.rs`, is:

* `the_claude_code_mcp_launcher_starts_the_installed_binary_with_no_path_entry`
  installs a real archive, then runs `integrations/claude-code/scripts/sure-mcp.ps1`
  with `SURE_BIN` removed and `PATH` set to a directory with nothing in it, and
  requires a successful MCP `initialize` and `tools/list` — then removes the
  installed `bin\sure.exe` and requires the same run to fail, so that the passing
  run cannot have been measuring something else.
* `the_install_flow_never_reaches_for_a_service_an_administrator_or_the_machine`
  reads both scripts' shipped code and fails on `New-Service`,
  `Register-ScheduledTask`, `HKLM:`, `HKEY_LOCAL_MACHINE`, `-Verb RunAs`, a
  `'Machine'` scope, or a `SetEnvironmentVariable` that is not inside a line that
  prints it.

## Uninstalling

```powershell
& .\scripts\Uninstall-Sure.ps1                       # the program; your history is kept
& .\scripts\Uninstall-Sure.ps1 -RemoveUserData       # the history too
& .\scripts\Uninstall-Sure.ps1 -WhatIf               # print the removals, make none
```

### It is structurally incapable of removing your evidence

Removing `%LOCALAPPDATA%\SURE` recursively would delete `sure.db` — the record
SURE exists to keep — while appearing to be tidying up. The uninstaller is built
so that it cannot:

1. **There is no recursive delete in the file.** A directory is removed only when
   it is already empty, so the filesystem refuses when anything the install did
   not write is inside it. The test reads the file and fails if the word
   `-Recurse` appears in it.
2. **A file is removed only when the install's own manifest records it *and* its
   digest still matches what the install wrote.** A file whose bytes changed
   since — a newer binary, something you replaced — is reported and left.
3. **`sure.db` is not in the manifest**, because the install never writes it, so
   the removal in (2) cannot reach it whatever it contains. The only path that
   can remove it is `-RemoveUserData`, which names it and defaults to off.
4. **With no manifest, it removes nothing at all** and exits `2`.

`-RemoveUserData` removes exactly `sure.db`, `sure.db-wal` and `sure.db-shm`, by
name, inside the install root. The names are `sure_core::paths::STORE_FILE` plus
SQLite's two WAL sidecars, and a test compares the list in the script against
that constant, so a rename on either side reddens rather than silently missing.

### When the directory holds files the install never wrote

This is the real case, and the one with no manifest to consult. The uninstaller
lists what it found, says that which files the install created cannot be
established, removes nothing, and exits `2`:

```text
== What this install created
   manifest     none found, so which files the install created cannot be established

   Files under ...\SURE, all of them left alone:
     bin\
     sure.db

   There is a program at ...\SURE\bin\sure.exe and no manifest that says this
   install put it there. It may have come from somewhere else.

   Nothing was removed. Removing files here would mean deleting by guesswork,
   in a directory that also holds the history SURE exists to keep. If the
   program is one you want gone, remove the files named above yourself.
```

`Cannot confirm` is a result, and it is not a yes.

### What it leaves behind

* The settings file `%APPDATA%\SURE\sure.yaml` is outside the install root and is
  never removed. The uninstaller names it and tells you to delete it yourself.
* `%LOCALAPPDATA%\SURE` itself is left in place even when it is empty: it is the
  per-user data directory for this application, and SURE will use it again.
* No `PATH` entry is removed, because none was added.

### `-WhatIf`

`-WhatIf` prints every removal it would make and makes none. The sentences
differ for the three outcomes — `removed`, `would remove … (-WhatIf: nothing was
removed)`, `not removed … (the removal was not confirmed)` — so a dry run cannot
end with "SURE is uninstalled". PowerShell's own "What if:" lines that precede
each one come from `ShouldProcess` and are localized by the system; the
surrounding sentences are this script's.

## What is not covered

Named as limits rather than left to be assumed:

* **The tests install a staged archive, not a release build.** They build the
  documented layout around the binary cargo just compiled, because
  `Build-Release.ps1` refuses to package without the release gate and takes
  minutes. The installer reads a real ZIP with real bytes; what is not exercised
  is a `Build-Release.ps1` artifact specifically. `P15-T002`'s verification of
  that artifact is separate from this.
* **The SmartScreen warning is not exercised.** No test runs an unsigned binary
  through a download-marked file and observes the prompt. The claim is limited
  to: the build carries no Authenticode signature, and Windows may warn.
* **No test asserts what an antivirus or an endpoint agent does** with the
  installed binary.
* **`Get-FileHash` is deliberately not used** by either script, and the reason is
  measured rather than imagined. It is a *script function* from
  `Microsoft.PowerShell.Utility`, so it only exists when the host autoloads the
  copy of that module which belongs to it. When a PowerShell 7 session sets
  `PSModulePath` and then starts Windows PowerShell 5.1 — which is what a
  launcher, a hook or a build script does — 5.1 resolves that module to the
  PowerShell 7 directory, it does not load, and `Get-FileHash` is absent while
  every other command either script uses still resolves as a *cmdlet*. The
  failure lands on the integrity check and reads like a typo. Both scripts use
  `[System.Security.Cryptography.SHA256]`, which is the same algorithm from the
  same place and depends on no module being findable.
* **Both hosts are driven, but only the ones this machine has.** The suite runs
  the whole flow under Windows PowerShell, which is always installed, and under
  PowerShell 7 when it can find `pwsh.exe` on `PATH` or at its standard install
  location. A machine without PowerShell 7 only tests the first.
* **`sure --store-dir` is not part of this flow.** The install always lands in
  the per-user location; a store elsewhere is a `sure` option and is documented
  with the CLI.

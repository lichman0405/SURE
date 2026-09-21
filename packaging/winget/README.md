# `packaging/winget/`

A template for a WinGet manifest, and the script that renders it from a release
archive.

**Nothing here is published.** No package has been submitted to
`microsoft/winget-pkgs`, and `winget install` does not install SURE today.

## Template

`template/` holds three files, because a WinGet manifest is three files:

```text
template/lichman0405.SURE.yaml               ManifestType: version
template/lichman0405.SURE.installer.yaml     ManifestType: installer
template/lichman0405.SURE.locale.en-US.yaml  ManifestType: defaultLocale
```

**Do not read a digest out of these files, and do not hand-edit a rendered
manifest.** The templates carry two placeholders — `<version>` and
`<sha256-of-the-archive>` — and no digest at all: the digest is computed from the
bytes of one archive by one script, and a plausible-looking 64-hex constant typed
into a committed YAML is exactly the artefact this project exists to refuse.

The templates carry a block of notes, fenced by two lines, that is removed before
anything is published. It says where each value comes from and what checks it.

## Rendering

```powershell
& .\scripts\New-WingetManifest.ps1 -Archive target\tmp\release\sure-<version>-x86_64-pc-windows-msvc.zip
```

It writes the rendered manifests to
`target\tmp\winget\lichman0405.SURE\<version>\`, then re-derives every value from
the archive and runs `winget validate --manifest` on the result. Exit `0` means
both passed; exit `1` means a value did not match the archive; exit `3` means
`winget` was not on `PATH`, so the schema check printed `--- NOT CHECKED ---`
instead of an OK.

```powershell
& .\scripts\New-WingetManifest.ps1 -Phase Verify -ManifestDirectory <dir> -Archive <archive>
```

`-Phase Verify` re-derives every value from an already-rendered directory, writes
nothing, and is what a reviewer or a CI job should run.

## Read this before touching any of it

`docs/development/INSTALL_WINGET.md` is the document: what the package installs,
where the files land, which of the launchers' three resolution steps finds it and
how that was measured, every value's provenance, what a fork has to change, the
update process and which parts of it are manual, what an uninstall removes and
what it leaves, and what none of this covers. `scripts/Build-Release.ps1` and
`docs/development/RELEASE_PROCESS.md` produce the archive it renders from.

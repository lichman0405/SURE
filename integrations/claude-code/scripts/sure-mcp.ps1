# SURE MCP server launcher for Claude Code.
#
# Claude Code starts this script as its `sure` MCP server and speaks the Model
# Context Protocol to it on stdin and stdout. That stdout is the protocol
# stream, so nothing here writes to it: what this script has to say to a person
# goes to stderr, and only the server's own messages go to the caller.
$ErrorActionPreference = 'SilentlyContinue'

# Resolve the binary the way the hook launcher does: override, PATH, per-user.
$bin = $env:SURE_BIN
if ($bin -and -not (Test-Path -LiteralPath $bin)) {
    [Console]::Error.WriteLine("SURE MCP server cannot start: SURE_BIN names '$bin', and nothing is there. Point it at the full path of sure.exe, or unset it to use PATH and %LOCALAPPDATA%\SURE\bin.")
    exit 3
}
if (-not $bin) { $found = Get-Command sure -ErrorAction SilentlyContinue; if ($found) { $bin = $found.Source } }
if (-not $bin) { $root = $env:LOCALAPPDATA; if (-not $root) { $root = [Environment]::GetFolderPath('LocalApplicationData') }; $perUser = Join-Path $root 'SURE\bin\sure.exe'; if (Test-Path -LiteralPath $perUser) { $bin = $perUser } }

# Fail closed, which is the opposite of what the hooks do. A hook exits 0 when
# SURE is missing so the agent is not blocked; a server that started without a
# binary would offer an empty tool list, which reads as SURE having looked.
if (-not $bin) {
    [Console]::Error.WriteLine('SURE MCP server cannot start: no SURE binary was found. It looked for the environment variable SURE_BIN, then for `sure` on PATH, then at %LOCALAPPDATA%\SURE\bin\sure.exe. Install SURE there, add it to PATH, or set SURE_BIN to the full path of sure.exe, then start the server again. SURE has checked nothing, and this session has no `sure` tools.')
    exit 3
}

# A build older than the bridge has no `mcp serve` and answers this with a usage
# error, so ask first: a server that dies on the caller's first message is worse.
$null = & $bin mcp serve --help 2>$null
$status = $LASTEXITCODE
if ($null -eq $status) { $why = 'could not be run at all' } else { $why = "does not carry 'sure mcp serve' (it answered with status $status)" }
if ($status -ne 0) {
    [Console]::Error.WriteLine("SURE MCP server cannot start: the binary at '$bin' $why, so it is not a build this package can talk to. This package needs a build that speaks Model Context Protocol revision 2025-11-25. Update SURE, or point SURE_BIN at a build that has the bridge.")
    exit 3
}

# Hand the session over: stdin, stdout and stderr pass through unchanged, and
# the server's own exit status is the one the caller sees.
& $bin mcp serve
exit $LASTEXITCODE

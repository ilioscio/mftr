# Set the version everywhere it is recorded: the workspace (Cargo.toml), the lock file (CI and
# release builds use --locked, so it must match) and the macOS export preset. The PowerShell twin
# of bump-version.sh, for Windows (Windows PowerShell 5.1 or PowerShell 7).
#   powershell -ExecutionPolicy Bypass -File scripts\bump-version.ps1 0.2.0
param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$') {
    [Console]::Error.WriteLine("not a version: $Version (expected X.Y.Z)")
    exit 1
}

# Rewrites a file in place as UTF-8 without a BOM, keeping its line endings.
function Set-InFile([string]$Path, [string]$Pattern, [string]$Replacement) {
    $full = (Resolve-Path $Path).Path
    $text = [System.IO.File]::ReadAllText($full)
    $new = [regex]::Replace($text, $Pattern, $Replacement, 'Multiline')
    [System.IO.File]::WriteAllText($full, $new, (New-Object System.Text.UTF8Encoding($false)))
}

Set-InFile 'Cargo.toml' '^version = "[^"]*"' "version = `"$Version`""
Set-InFile 'client/export_presets.cfg' '^(application/(short_)?version)="[^"]*"' "`$1=`"$Version`""
# Rewrites only our own crates' entries; dependencies stay pinned.
cargo update --workspace --offline
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Version $Version. Commit, merge to main, then tag that merge: git tag v$Version; git push origin v$Version"

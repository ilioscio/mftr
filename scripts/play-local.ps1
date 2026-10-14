<#
.SYNOPSIS
    Build and run a local MFTR server and client from this checkout, for testing a branch.

.DESCRIPTION
    Builds the client's GDExtension (debug, which the Godot editor/runner loads from
    target/debug) and the server (release), starts the server in its own window, then runs the
    Godot client against it. When the client closes, the server is stopped and any `.uid` files
    Godot generated during the run are removed (a newer editor than the project's makes them).

.EXAMPLE
    .\scripts\play-local.ps1                 # build, start the server, open the client's menu
    .\scripts\play-local.ps1 -Join           # skip the menu: connect to the local server at once
    .\scripts\play-local.ps1 -Editor        # open the project in the Godot editor instead
    .\scripts\play-local.ps1 -NoServer      # a server is already running: just the client
    .\scripts\play-local.ps1 -MapEditor     # the map editor (no server)
    scripts\play.cmd -MapEditor             # the same, whatever PowerShell's script policy
    .\scripts\play-local.ps1 -ServerArgs '--scenario','duel'
#>
param(
    [int]$Port = 7777,
    [switch]$Join,
    [switch]$Editor,
    [switch]$NoServer,
    [switch]$MapEditor,
    [switch]$NoBuild,
    [switch]$KeepUids,
    [string]$Godot = $env:GODOT,
    [string[]]$ServerArgs = @(),
    [string[]]$ClientArgs = @()
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$client = Join-Path $root 'client'
$address = "127.0.0.1:$Port"
if ($MapEditor) {
    $NoServer = $true
    $ClientArgs = @('--map-editor') + $ClientArgs
}

# Godot: -Godot, $env:GODOT (scripts\setup.cmd saves it), `godot` on PATH, or the newest install
# in the usual folders or winget's.
if (-not $Godot) {
    $Godot = [Environment]::GetEnvironmentVariable('GODOT', 'User')
}
if (-not $Godot) {
    $Godot = (Get-Command godot, godot4 -ErrorAction SilentlyContinue | Select-Object -First 1).Source
}
if (-not $Godot) {
    $folders = @("$env:ProgramFiles\Godot", "${env:ProgramFiles(x86)}\Godot", "$env:LOCALAPPDATA\Godot",
        "$env:LOCALAPPDATA\Programs\Godot", "$env:USERPROFILE\Godot")
    $folders += @(Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter 'GodotEngine.GodotEngine*' `
        -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
    $Godot = Get-ChildItem $folders `
        -Recurse -Filter 'Godot_v*_win64.exe' -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -notlike '*console*' } |
        Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Godot -or -not (Test-Path $Godot)) {
    throw "Godot not found: run scripts\setup.cmd, pass -Godot <path to Godot_v4.x_win64.exe> or set `$env:GODOT."
}

Write-Host "MFTR checkout: $root ($(git -C $root branch --show-current))"

if (-not $NoBuild) {
    Write-Host 'Building the client library (debug)...'
    cargo build --manifest-path "$root\Cargo.toml" -p mftr-gdext
    if ($LASTEXITCODE -ne 0) { throw 'client library build failed' }
    if (-not $NoServer) {
        Write-Host 'Building the server (release)...'
        cargo build --release --manifest-path "$root\Cargo.toml" -p mftr-server
        if ($LASTEXITCODE -ne 0) { throw 'server build failed' }
    }
}

$server = $null
if (-not $NoServer) {
    if (Get-NetUDPEndpoint -LocalPort $Port -ErrorAction SilentlyContinue) {
        throw "UDP port $Port is already in use (a server still running?). Stop it, or use -NoServer or -Port."
    }
    $exe = Join-Path $root 'target\release\mftr-server.exe'
    Write-Host "Starting the server on $address (its own window)..."
    $server = Start-Process -FilePath $exe -ArgumentList (@('--bind', $address) + $ServerArgs) `
        -WorkingDirectory $root -PassThru
    Start-Sleep -Seconds 1
    if ($server.HasExited) { throw "the server exited at once (code $($server.ExitCode))" }
}

# Untracked .uid files before the run, so only the ones Godot makes now are cleaned up.
$uidsBefore = @(git -C $root ls-files --others --exclude-standard -- '*.uid')

try {
    if ($Editor) {
        Write-Host "Opening the editor (press Play, then join $address)..."
        & $Godot --editor --path $client | Out-Null
    } else {
        $userArgs = @()
        if ($Join) { $userArgs += $address }
        $userArgs += $ClientArgs
        $godotArgs = @('--path', $client)
        if ($userArgs.Count -gt 0) { $godotArgs += @('--') + $userArgs }
        Write-Host ("Running the client" + $(if ($Join) { " (joining $address)" } else { " (menu: join $address)" }) + '...')
        & $Godot @godotArgs | Out-Null
    }
} finally {
    if ($server -and -not $server.HasExited) {
        Write-Host 'Stopping the server...'
        Stop-Process -Id $server.Id -Force
    }
    if (-not $KeepUids) {
        $new = @(git -C $root ls-files --others --exclude-standard -- '*.uid') | Where-Object { $_ -notin $uidsBefore }
        foreach ($f in $new) { Remove-Item (Join-Path $root $f) -Force }
        if ($new.Count -gt 0) { Write-Host "Removed $($new.Count) generated .uid file(s)." }
    }
}

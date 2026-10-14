<#
.SYNOPSIS
    Set up a Windows machine to develop MFTR, or check one that is set up already.

.DESCRIPTION
    Checks each tool the project needs: Git, the Microsoft C++ build tools (Rust links with
    them), Rust (rustup, with the toolchain rust-toolchain.toml names), Godot and, with -Art,
    Blender. Offers to install what's missing with winget, asking first. Remembers where Godot
    is (the GODOT user variable, which play-local.ps1 reads). Then builds the client library
    and the server once and imports the Godot project, so the first `scripts\play.cmd` starts
    quickly. Run it again any time to see what's missing or out of date.

    Run it through scripts\setup.cmd, which works whatever PowerShell's script policy is.

.EXAMPLE
    scripts\setup.cmd             # check, offer installs, build
    scripts\setup.cmd -Yes        # install whatever is missing without asking
    scripts\setup.cmd -Art        # also Blender, for champion models and animations
    scripts\setup.cmd -NoBuild    # only check and install the tools
#>
param(
    [switch]$Yes,
    [switch]$Art,
    [switch]$NoBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent

# The Godot the team edits with (docs/setup.md). Any 4.5 or newer runs the project, but one
# version for everyone keeps Godot from rewriting files another version saved.
$GodotVersion = '4.7.2'
$GodotMinimum = [version]'4.5'
$BlenderVersion = '5.2'

$rows = New-Object System.Collections.ArrayList
function Report([string]$Tool, [string]$State, [string]$Detail) {
    [void]$rows.Add([pscustomobject]@{ Tool = $Tool; State = $State; Detail = $Detail })
    $color = @{ ok = 'Green'; missing = 'Red'; warning = 'Yellow'; optional = 'DarkGray' }[$State]
    Write-Host ("  {0,-18} {1,-9} {2}" -f $Tool, $State, $Detail) -ForegroundColor $color
}

function Ask([string]$Question) {
    if ($Yes) { return $true }
    $answer = Read-Host "$Question [Y/n]"
    return ($answer -eq '' -or $answer -match '^[yY]')
}

# A new install changes PATH in the registry, not in this window: read it again.
function Update-Path {
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' +
        [Environment]::GetEnvironmentVariable('Path', 'User')
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) { $env:Path += ";$cargoBin" }
}

# Installs a winget package after asking. Returns $true if it was installed.
function Install-Package([string]$Name, [string]$Id, [string[]]$Extra = @()) {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Write-Host "    winget isn't available: install $Name by hand (docs/setup.md links it)." -ForegroundColor Yellow
        return $false
    }
    if (-not (Ask "    Install $Name with winget (this accepts its license)?")) { return $false }
    winget install --id $Id --exact --accept-package-agreements --accept-source-agreements @Extra
    $ok = $LASTEXITCODE -eq 0
    if (-not $ok) { Write-Host "    winget couldn't install $Name (exit $LASTEXITCODE)." -ForegroundColor Yellow }
    Update-Path
    return $ok
}

function Find-Msvc {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { return $null }
    $path = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($path) { return $path } else { return $null }
}

# Every Godot we can see, newest first: GODOT, PATH, the usual folders and winget's.
function Find-Godots {
    $found = @()
    if ($env:GODOT -and (Test-Path $env:GODOT)) { $found += (Get-Item $env:GODOT) }
    foreach ($c in Get-Command godot, godot4 -ErrorAction SilentlyContinue) {
        if ($c.Source -and (Test-Path $c.Source)) { $found += (Get-Item $c.Source) }
    }
    $folders = @("$env:ProgramFiles\Godot", "${env:ProgramFiles(x86)}\Godot", "$env:LOCALAPPDATA\Godot",
        "$env:LOCALAPPDATA\Programs\Godot", "$env:USERPROFILE\Godot")
    $folders += @(Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter 'GodotEngine.GodotEngine*' `
        -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
    $found += @(Get-ChildItem $folders -Recurse -Filter 'Godot_v*_win64.exe' -ErrorAction SilentlyContinue)
    $found | Where-Object { $_.Name -notlike '*console*' -and $_.Name -notlike '*mono*' } |
        Sort-Object FullName -Unique |
        ForEach-Object {
            $v = $null
            if ($_.Name -match 'Godot_v(\d+\.\d+(\.\d+)?)') { $v = [version]$Matches[1] }
            [pscustomobject]@{ Path = $_.FullName; Version = $v }
        } |
        Sort-Object Version -Descending
}

Write-Host "MFTR setup: $root"
Write-Host ''

# ---- Checkout path --------------------------------------------------------------------------
# The build writes paths up to ~135 characters inside the checkout, and the C++ linker can't
# open a path over Windows' 260 (long path support doesn't reach it): the build fails with
# LNK1104. Keep the checkout short.
$pathOk = $root.Length -le 110
if ($pathOk) {
    Report 'Checkout path' 'ok' "$($root.Length) characters"
} else {
    Report 'Checkout path' 'warning' "$($root.Length) characters: the build will likely fail (LNK1104). Clone to a short path such as C:\dev\mftr"
}

# ---- Git ------------------------------------------------------------------------------------
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host '  Git is missing (it also brings Git Bash, for the scripts\*.sh tools).'
    [void](Install-Package 'Git' 'Git.Git')
}
if (Get-Command git -ErrorAction SilentlyContinue) {
    Report 'Git' 'ok' ((git --version) -replace '^git version ', '')
} else {
    Report 'Git' 'missing' 'https://git-scm.com/download/win'
}

# ---- C++ build tools ------------------------------------------------------------------------
$msvc = Find-Msvc
if (-not $msvc) {
    Write-Host '  The Microsoft C++ build tools are missing: Rust needs their linker on Windows.'
    Write-Host '    (A few GB; the installer shows its own progress window.)'
    [void](Install-Package 'Visual Studio Build Tools (C++)' 'Microsoft.VisualStudio.2022.BuildTools' `
        @('--override', '--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'))
    $msvc = Find-Msvc
}
if ($msvc) {
    Report 'C++ build tools' 'ok' $msvc
} else {
    Report 'C++ build tools' 'missing' 'Visual Studio Installer > Build Tools > Modify > "Desktop development with C++"'
}

# ---- Rust -----------------------------------------------------------------------------------
Update-Path
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host '  Rust (rustup) is missing.'
    [void](Install-Package 'Rust (rustup)' 'Rustlang.Rustup')
}
$rustOk = $false
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    # The channel rust-toolchain.toml names, with its components (cargo uses it in this checkout).
    $toml = Get-Content (Join-Path $root 'rust-toolchain.toml') -Raw
    $channel = if ($toml -match 'channel\s*=\s*"([^"]+)"') { $Matches[1] } else { 'stable' }
    $components = if ($toml -match 'components\s*=\s*\[([^\]]*)\]') {
        @([regex]::Matches($Matches[1], '"([^"]+)"') | ForEach-Object { $_.Groups[1].Value })
    } else { @() }
    $installArgs = @('toolchain', 'install', $channel, '--profile', 'minimal')
    foreach ($c in $components) { $installArgs += @('--component', $c) }
    Write-Host "  Checking the Rust toolchain ($channel)..."
    & rustup @installArgs
    Push-Location $root
    try { $host_ = @(& rustc -vV) | Where-Object { $_ -like 'host:*' } | ForEach-Object { $_ -replace '^host:\s*', '' } } finally { Pop-Location }
    if (-not $host_) {
        Report 'Rust' 'missing' "rustup is installed but the $channel toolchain isn't working: run 'rustup toolchain install $channel'"
    } elseif ($host_ -notlike '*msvc*') {
        Report 'Rust' 'warning' "host $host_ - MFTR builds with the MSVC toolchain: rustup default $channel-x86_64-pc-windows-msvc"
        $rustOk = $true
    } else {
        Push-Location $root
        try { $ver = (& rustc --version) -replace '^rustc ', '' } finally { Pop-Location }
        Report 'Rust' 'ok' "$ver ($channel, $host_)"
        $rustOk = $true
    }
} else {
    Report 'Rust' 'missing' 'https://rustup.rs'
}

# ---- Godot ----------------------------------------------------------------------------------
$godot = Find-Godots | Where-Object { $_.Version -and $_.Version -ge $GodotMinimum } | Select-Object -First 1
$wanted = Find-Godots | Where-Object { $_.Version -eq [version]$GodotVersion } | Select-Object -First 1
if ($wanted) { $godot = $wanted }
if (-not $godot) {
    Write-Host "  Godot $GodotMinimum or newer is missing."
    if (Install-Package "Godot $GodotVersion" 'GodotEngine.GodotEngine' @('--version', $GodotVersion)) {
        $godot = Find-Godots | Where-Object { $_.Version -eq [version]$GodotVersion } | Select-Object -First 1
    }
}
if ($godot) {
    # Remember it for play-local.ps1 (and new terminals): the GODOT user variable.
    $saved = [Environment]::GetEnvironmentVariable('GODOT', 'User')
    if ($saved -ne $godot.Path) {
        if (Ask "    Remember Godot at $($godot.Path) (sets the GODOT user variable)?") {
            [Environment]::SetEnvironmentVariable('GODOT', $godot.Path, 'User')
        }
    }
    $env:GODOT = $godot.Path
    if ($godot.Version -eq [version]$GodotVersion) {
        Report 'Godot' 'ok' "$($godot.Version) $($godot.Path)"
    } else {
        Report 'Godot' 'warning' "$($godot.Version) runs it, but the team edits with $GodotVersion (winget install GodotEngine.GodotEngine --version $GodotVersion)"
    }
} else {
    Report 'Godot' 'missing' "Godot $GodotVersion (standard, not .NET): https://godotengine.org/download/archive/$GodotVersion-stable/"
}

# ---- Blender (art only) ---------------------------------------------------------------------
$blender = Join-Path $env:ProgramFiles "Blender Foundation\Blender $BlenderVersion\blender.exe"
if (-not (Test-Path $blender) -and $Art) {
    Write-Host "  Blender $BlenderVersion is missing (champion models and animations, art/README.md)."
    [void](Install-Package "Blender $BlenderVersion" 'BlenderFoundation.Blender')
}
if (Test-Path $blender) {
    Report 'Blender' 'ok' $blender
} elseif ($Art) {
    Report 'Blender' 'missing' "Blender $($BlenderVersion): https://www.blender.org/download/"
} else {
    Report 'Blender' 'optional' "only for art; scripts\setup.cmd -Art installs it"
}

# ---- First build ----------------------------------------------------------------------------
$built = $false
if (-not $NoBuild -and $rustOk -and $msvc) {
    Write-Host ''
    Write-Host 'Building the client library (debug) and the server (release); the first build takes a few minutes...'
    cargo build --manifest-path "$root\Cargo.toml" -p mftr-gdext
    if ($LASTEXITCODE -eq 0) { cargo build --release --manifest-path "$root\Cargo.toml" -p mftr-server }
    if ($LASTEXITCODE -eq 0) {
        Report 'Build' 'ok' 'target\debug\mftr_gdext.dll, target\release\mftr-server.exe'
        $built = $true
    } else {
        Report 'Build' 'missing' 'cargo build failed: see the errors above'
    }
}
if ($built -and $godot) {
    Write-Host 'Importing the Godot project (textures, models)...'
    $uidsBefore = @(git -C $root ls-files --others --exclude-standard -- '*.uid')
    $console = $godot.Path -replace '\.exe$', '_console.exe'
    $exe = if (Test-Path $console) { $console } else { $godot.Path }
    # Its log goes to a temporary file: Windows PowerShell turns a native program's stderr into
    # errors, and the import's chatter isn't worth reading.
    $log = [IO.Path]::GetTempFileName()
    $proc = Start-Process -FilePath $exe -ArgumentList @('--headless', '--path', ('"' + (Join-Path $root 'client') + '"'), '--import') `
        -NoNewWindow -Wait -PassThru -RedirectStandardOutput $log -RedirectStandardError "$log.err"
    $code = $proc.ExitCode
    Remove-Item $log, "$log.err" -Force -ErrorAction SilentlyContinue
    # A Godot newer than the files' makes .uid files beside them: they aren't ours to commit.
    $new = @(git -C $root ls-files --others --exclude-standard -- '*.uid') | Where-Object { $_ -notin $uidsBefore }
    foreach ($f in $new) { Remove-Item (Join-Path $root $f) -Force }
    if ($code -eq 0) { Report 'Godot import' 'ok' 'client\.godot' } else { Report 'Godot import' 'warning' "exit $($code): opening the project in the editor imports it too" }
}

# ---- Summary --------------------------------------------------------------------------------
Write-Host ''
$missing = @($rows | Where-Object State -eq 'missing')
if ($missing.Count -gt 0) {
    Write-Host "Not ready: $($missing.Count) missing ($(($missing | ForEach-Object Tool) -join ', ')). Install them, then run scripts\setup.cmd again." -ForegroundColor Red
} else {
    Write-Host 'Ready. Next:' -ForegroundColor Green
    Write-Host '  scripts\play.cmd                 server + client (pick a mode in the menu)'
    Write-Host '  scripts\play.cmd -MapEditor      the map editor'
    Write-Host '  scripts\play.cmd -Editor         the Godot editor on the project'
    Write-Host '  cargo test                       the Rust tests'
    Write-Host '  More: docs\setup.md'
    if ($env:GODOT -and [Environment]::GetEnvironmentVariable('GODOT', 'User') -eq $env:GODOT) {
        Write-Host '  (Open a new terminal for GODOT to be set there too.)' -ForegroundColor DarkGray
    }
}

# Started by a double-click: keep the window open to read the result.
try {
    $parent = (Get-CimInstance Win32_Process -Filter "ProcessId=$PID").ParentProcessId
    $grand = (Get-CimInstance Win32_Process -Filter "ProcessId=$parent").ParentProcessId
    if ((Get-Process -Id $grand -ErrorAction SilentlyContinue).Name -eq 'explorer') { [void](Read-Host 'Press Enter to close') }
} catch { }

if ($missing.Count -gt 0) { exit 1 }

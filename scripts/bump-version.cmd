@echo off
rem Set the version everywhere it is recorded (see bump-version.ps1).
rem Runs scripts\bump-version.ps1 through Windows PowerShell for this one run, whatever its script policy, so
rem it works from cmd, PowerShell or a double-click. Arguments pass through: bump-version.cmd 0.12.0
powershell -NoProfile -ExecutionPolicy Bypass -Command "& '%~dp0bump-version.ps1' %*; exit $LASTEXITCODE"

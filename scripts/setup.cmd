@echo off
rem Set up (or check) this Windows machine for MFTR development: docs\setup.md.
rem Runs scripts\setup.ps1 through Windows PowerShell for this one run, whatever its script policy, so
rem it works from cmd, PowerShell or a double-click. Arguments pass through: setup.cmd -Yes
powershell -NoProfile -ExecutionPolicy Bypass -Command "& '%~dp0setup.ps1' %*; exit $LASTEXITCODE"

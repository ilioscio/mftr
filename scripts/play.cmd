@echo off
rem Build and run a local server and client from this checkout (or the map editor, or Godot).
rem Runs scripts\play-local.ps1 through Windows PowerShell for this one run, whatever its script policy, so
rem it works from cmd, PowerShell or a double-click. Arguments pass through: play.cmd -MapEditor
powershell -NoProfile -ExecutionPolicy Bypass -Command "& '%~dp0play-local.ps1' %*; exit $LASTEXITCODE"

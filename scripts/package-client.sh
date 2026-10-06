#!/usr/bin/env bash
# Export the Godot client for one platform into dist/ as a ready-to-play archive (M2 slice 6).
#
#   scripts/package-client.sh linux|windows|macos [GODOT]
#
# Needs: the Godot 4.5 editor (GODOT, default `godot` on PATH) with its 4.5 export templates
# installed, and the platform's Godot extension built with `cargo build --profile dist -p
# mftr-gdext` (for another OS, put its library into client/bin/ yourself; CI does that).
set -euo pipefail

platform="${1:?usage: $0 linux|windows|macos [GODOT]}"
godot="${2:-${GODOT:-godot}}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

case "$platform" in
  linux)   preset="Linux";   lib="libmftr_gdext.so";    out="dist/client/linux/mftr.x86_64" ;;
  windows) preset="Windows"; lib="mftr_gdext.dll";      out="dist/client/windows/mftr.exe" ;;
  macos)   preset="macOS";   lib="libmftr_gdext.dylib"; out="dist/client/macos/mftr.zip" ;;
  *) echo "unknown platform $platform" >&2; exit 2 ;;
esac

mkdir -p client/bin "$(dirname "$out")"
if [ -f "target/dist/$lib" ]; then
  cp "target/dist/$lib" client/bin/
fi
[ -f "client/bin/$lib" ] || { echo "missing client/bin/$lib (build mftr-gdext with --profile dist)" >&2; exit 1; }

# The first headless run imports the project (creates client/.godot), then export.
"$godot" --headless --path client --import >/dev/null 2>&1 || true
"$godot" --headless --path client --export-release "$preset" "../$out"

version="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
archive="dist/mftr-client-$version-$platform"
case "$platform" in
  macos) cp "$out" "$archive.zip"; echo "$archive.zip" ;;
  windows) (cd "$(dirname "$out")" && zip -qr "$root/$archive.zip" .); echo "$archive.zip" ;;
  *) tar -C "$(dirname "$out")" -czf "$archive.tar.gz" .; echo "$archive.tar.gz" ;;
esac

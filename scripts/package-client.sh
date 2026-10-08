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

# The champion packs (A3, A4): the client reads them from `art/` next to the executable through
# mftr-pack, never as Godot resources, so they ship as plain files beside the game: every
# `export/` folder of the shared library, the champions, the lane minions and the map props (models, clips, VFX,
# sounds), and the item icons (SVG).
copy_packs() {
  local dest="$1"
  rm -rf "$dest"
  for d in art/library/*/export art/champions/*/export art/minions/*/export art/props/*/export art/items/icons; do
    [ -d "$d" ] || continue
    mkdir -p "$dest/${d#art/}"
    cp -R "$d/." "$dest/${d#art/}/"
  done
  [ -f "$dest/library/biped/export/biped_library.glb" ] || { echo "packs missing from $dest" >&2; exit 1; }
}
case "$platform" in
  macos)
    # Into the app bundle: unpack the export, add Contents/Resources/art, pack it again.
    tmp="$(mktemp -d)"
    (cd "$tmp" && unzip -q "$root/$out")
    app="$(cd "$tmp" && ls -d ./*.app | head -1)"
    [ -n "$app" ] || { echo "no .app in $out" >&2; exit 1; }
    copy_packs "$tmp/$app/Contents/Resources/art"
    rm -f "$out"
    (cd "$tmp" && zip -qry "$root/$out" .)
    rm -rf "$tmp"
    ;;
  *) copy_packs "$(dirname "$out")/art" ;;
esac

version="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
archive="dist/mftr-client-$version-$platform"
case "$platform" in
  macos) cp "$out" "$archive.zip"; echo "$archive.zip" ;;
  windows) (cd "$(dirname "$out")" && zip -qr "$root/$archive.zip" .); echo "$archive.zip" ;;
  *) tar -C "$(dirname "$out")" -czf "$archive.tar.gz" .; echo "$archive.tar.gz" ;;
esac

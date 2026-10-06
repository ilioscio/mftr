#!/usr/bin/env bash
# Set the version everywhere it is recorded: the workspace (Cargo.toml), the lock file (CI and
# release builds use --locked, so it must match) and the macOS export preset.
#   scripts/bump-version.sh 0.2.0
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/bump-version.sh X.Y.Z}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a version: $version" >&2; exit 1; }

sed -i.bak -E "s/^version = \"[^\"]*\"/version = \"$version\"/" Cargo.toml
sed -i.bak -E "s/^(application\/(short_)?version)=\"[^\"]*\"/\1=\"$version\"/" client/export_presets.cfg
rm -f Cargo.toml.bak client/export_presets.cfg.bak
# Rewrites only our own crates' entries; dependencies stay pinned.
cargo update --workspace --offline

echo "Version $version. Commit, merge to main, then tag that merge: git tag v$version && git push origin v$version"

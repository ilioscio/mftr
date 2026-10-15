#!/usr/bin/env bash
# Tag a release on GitHub's main, never on a local checkout that may be behind: reads the version
# from origin/main's Cargo.toml (merge the bump-version PR first) and tags that exact commit.
#   scripts/tag-release.sh
set -euo pipefail
cd "$(dirname "$0")/.."

git fetch --quiet --tags origin main
commit=$(git rev-parse origin/main)
version=$(git show origin/main:Cargo.toml | grep -m1 '^version' | cut -d'"' -f2)
tag="v$version"
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "$tag already exists (on $(git rev-parse --short "$tag^{commit}")): bump first with scripts/bump-version.sh X.Y.Z" >&2
  exit 1
fi
git tag "$tag" "$commit"
git push origin "$tag"
echo "Tagged $tag on $(git rev-parse --short "$commit") ($(git log -1 --format=%s "$commit")). The release workflow builds it."

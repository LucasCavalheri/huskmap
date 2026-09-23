#!/usr/bin/env bash
# A release tag must match the workspace version and have a CHANGELOG section.
#   scripts/validate-release.sh <repo-root> <tag>
set -euo pipefail
ROOT="${1:?repo root}"
TAG="${2:?tag like v0.1.0}"
[[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] || { echo "tag must look like v1.2.3: $TAG" >&2; exit 1; }
VERSION="${TAG#v}"
CARGO="$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p}' "$ROOT/Cargo.toml" | head -n 1)"
[[ "$CARGO" == "$VERSION" ]] || { echo "tag $TAG but Cargo.toml says $CARGO" >&2; exit 1; }
BASE="${VERSION%%-*}"
grep -Eq "^## \[?v?${BASE//./\\.}\]?( |$)" "$ROOT/CHANGELOG.md" || { echo "CHANGELOG.md has no section for $BASE" >&2; exit 1; }
echo "release $TAG ok"

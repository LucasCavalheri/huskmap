#!/usr/bin/env bash
# validate-release.sh and generate-sha256sums.sh against throwaway repos.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
pass=0; ok() { pass=$((pass + 1)); }

mk() {
  mkdir -p "$W/$1"
  printf '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "%s"\n' "$2" > "$W/$1/Cargo.toml"
  printf '# Changelog\n\n## %s - 2026-09-23\n\n- first\n' "$3" > "$W/$1/CHANGELOG.md"
}
mk good 0.2.0 0.2.0
bash "$ROOT/scripts/validate-release.sh" "$W/good" v0.2.0 >/dev/null || fail "good release"; ok
mk pre 0.3.0-rc.1 "[0.3.0]"
bash "$ROOT/scripts/validate-release.sh" "$W/pre" v0.3.0-rc.1 >/dev/null || fail "prerelease"; ok
if bash "$ROOT/scripts/validate-release.sh" "$W/good" v0.2.1 2>/dev/null; then fail "version mismatch"; fi; ok
mk nolog 0.4.0 0.3.9
if bash "$ROOT/scripts/validate-release.sh" "$W/nolog" v0.4.0 2>/dev/null; then fail "missing changelog"; fi; ok
if bash "$ROOT/scripts/validate-release.sh" "$W/good" 0.2.0 2>/dev/null; then fail "tag without v"; fi; ok
bash "$ROOT/scripts/validate-release.sh" "$ROOT" "v$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p}' "$ROOT/Cargo.toml")" >/dev/null || fail "this repo"; ok

mkdir -p "$W/dist"; echo a > "$W/dist/b.deb"; echo b > "$W/dist/a.tar.gz"; echo old > "$W/dist/SHA256SUMS"
bash "$ROOT/scripts/generate-sha256sums.sh" "$W/dist" >/dev/null
[[ "$(wc -l < "$W/dist/SHA256SUMS")" -eq 2 ]] || fail "sums lines"; ok
( cd "$W/dist" && sha256sum -c SHA256SUMS >/dev/null ) || fail "sums verify"; ok
head -n1 "$W/dist/SHA256SUMS" | grep -q "a.tar.gz" || fail "sorted"; ok
mkdir -p "$W/empty"
if bash "$ROOT/scripts/generate-sha256sums.sh" "$W/empty" 2>/dev/null; then fail "empty dir"; fi; ok
echo "release tools: $pass checks passed"

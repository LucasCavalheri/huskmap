#!/usr/bin/env bash
# Build every package from a real binary and check what is inside.
#   scripts/package.test.sh target/release/huskmap [amd64|arm64]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${1:?usage: $0 <huskmap binary> [amd64|arm64]}"
ARCH="${2:-amd64}"
NATIVE=x86_64; [[ "$ARCH" == arm64 ]] && NATIVE=aarch64
TAR_ARCH=x64; [[ "$ARCH" == arm64 ]] && TAR_ARCH=arm64
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
pass=0; ok() { pass=$((pass + 1)); }

HUSKMAP_SKIP_RPM="${HUSKMAP_SKIP_RPM:-0}" "$ROOT/packaging/build-linux-packages.sh" "$BIN" "$VERSION" "$ARCH" "$OUT" >/dev/null

must_list() {
  local listing="$1"; shift
  for f in "$@"; do grep -q -- "$f" <<<"$listing" || fail "missing $f"; done
  ok
}
FILES=(usr/bin/huskmap usr/share/applications/lucas.cavalheri.huskmap.desktop
  usr/share/icons/hicolor/256x256/apps/huskmap.png usr/share/icons/hicolor/scalable/apps/huskmap.svg
  usr/share/bash-completion/completions/huskmap usr/share/zsh/site-functions/_huskmap
  usr/share/fish/vendor_completions.d/huskmap.fish usr/share/man/man1/huskmap.1.gz
  usr/share/licenses/huskmap/LICENSE)

# Debian
DEB="$OUT/huskmap_${VERSION}_${ARCH}.deb"
[[ "$(dpkg-deb -f "$DEB" Package)" == huskmap ]] || fail "deb Package"; ok
[[ "$(dpkg-deb -f "$DEB" Version)" == "$VERSION" ]] || fail "deb Version"; ok
[[ "$(dpkg-deb -f "$DEB" Architecture)" == "$ARCH" ]] || fail "deb Architecture"; ok
dpkg-deb -f "$DEB" Depends | grep -q libfontconfig1 || fail "deb Depends"; ok
must_list "$(dpkg-deb -c "$DEB")" "${FILES[@]}"
dpkg-deb -c "$DEB" | grep " ./usr/bin/huskmap" | grep -q "^-rwxr-xr-x root/root" || fail "deb binary mode/owner"; ok
dpkg-deb -x "$DEB" "$OUT/deb-x"
"$OUT/deb-x/usr/bin/huskmap" --version | grep -q "$VERSION" || fail "extracted binary"; ok
zcat "$OUT/deb-x/usr/share/man/man1/huskmap.1.gz" | grep -q '^\.TH huskmap' || fail "man page"; ok
grep -q "_huskmap" "$OUT/deb-x/usr/share/bash-completion/completions/huskmap" || fail "bash completion"; ok
if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$OUT/deb-x/usr/share/applications/lucas.cavalheri.huskmap.desktop" || fail "desktop file"
  ok
fi

# Arch
PAC="$OUT/huskmap-${VERSION}-1-${NATIVE}.pkg.tar.zst"
must_list "$(zstd -dc "$PAC" | tar -tf -)" .PKGINFO "${FILES[@]}"
zstd -dc "$PAC" | tar -xOf - .PKGINFO | grep -qx "pkgver = ${VERSION}-1" || fail "pacman pkgver"; ok
zstd -dc "$PAC" | tar -xOf - .PKGINFO | grep -qx "arch = ${NATIVE}" || fail "pacman arch"; ok

# Alpine
APK="$OUT/huskmap-${VERSION}-r0-${NATIVE}.apk"
must_list "$(tar -tzf "$APK")" .PKGINFO "${FILES[@]}"
tar -xzOf "$APK" .PKGINFO | grep -qx "depend = gcompat" || fail "apk gcompat"; ok

# Portable
TAR="$OUT/huskmap-linux-${TAR_ARCH}.tar.gz"
must_list "$(tar -tzf "$TAR")" huskmap/bin/huskmap huskmap/install.sh huskmap/share/applications/lucas.cavalheri.huskmap.desktop

# RPM, where rpm exists
RPM="$OUT/huskmap-${VERSION}-1.${NATIVE}.rpm"
if [[ -f "$RPM" ]]; then
  must_list "$(rpm -qpl "$RPM" 2>/dev/null)" /usr/bin/huskmap /usr/share/man/man1/huskmap.1.gz
  [[ "$(rpm -qp --qf '%{NAME} %{VERSION}' "$RPM" 2>/dev/null)" == "huskmap ${VERSION//-/\~}" ]] || fail "rpm name/version"
  ok
fi

# bad input is refused
if "$ROOT/packaging/build-linux-packages.sh" "$BIN" not-a-version "$ARCH" "$OUT/x" 2>/dev/null; then fail "bad version"; fi; ok
if "$ROOT/packaging/build-linux-packages.sh" "$BIN" 1.0.0 riscv "$OUT/x" 2>/dev/null; then fail "bad arch"; fi; ok
if "$ROOT/packaging/build-linux-packages.sh" /nonexistent 1.0.0 "$ARCH" "$OUT/x" 2>/dev/null; then fail "missing binary"; fi; ok

echo "packages: $pass checks passed"

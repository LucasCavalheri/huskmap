#!/usr/bin/env bash
# Build every Linux package from one release binary:
#   huskmap_<ver>_<deb-arch>.deb            Debian, Ubuntu, Mint, Pop!_OS, …
#   huskmap-<ver>-1.<rpm-arch>.rpm          Fedora, RHEL, openSUSE, …   (needs rpmbuild)
#   huskmap-<ver>-1-<arch>.pkg.tar.zst      Arch, Manjaro               (needs zstd)
#   huskmap-<ver>-r0-<arch>.apk             Alpine (with gcompat)
#   huskmap-linux-<x64|arm64>.tar.gz        everything else
#
# The binary must run on this host: completions and the man page come from it.
# HUSKMAP_SKIP_RPM=1 skips the rpm when rpmbuild is not installed (local runs).
set -euo pipefail

usage() {
  echo "usage: $0 <binary> <version> <amd64|arm64> <output-dir>" >&2
  exit 2
}

[[ $# -eq 4 ]] || usage

BINARY="$1"
VERSION="$2"
PACKAGE_ARCH="$3"
OUTPUT_DIR="$4"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="lucas.cavalheri.huskmap"
SUMMARY="Map the worktrees, sessions and ballast your coding agents left behind"
URL="https://github.com/LucasCavalheri/huskmap"
MAINTAINER="Lucas Cavalheri <lucas.dev.carvalho@gmail.com>"

case "$PACKAGE_ARCH" in
  amd64) DEB_ARCH=amd64; NATIVE_ARCH=x86_64; TAR_ARCH=x64 ;;
  arm64) DEB_ARCH=arm64; NATIVE_ARCH=aarch64; TAR_ARCH=arm64 ;;
  *) echo "unsupported package architecture: $PACKAGE_ARCH" >&2; exit 2 ;;
esac

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.]+)?$ ]] || {
  echo "not a version: $VERSION" >&2
  exit 2
}
[[ -x "$BINARY" ]] || { echo "binary not found or not executable: $BINARY" >&2; exit 1; }
"$BINARY" --version >/dev/null || { echo "binary does not run on this host: $BINARY" >&2; exit 1; }

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

populate_root() {
  local stage="$1"
  install -d \
    "$stage/usr/bin" \
    "$stage/usr/share/applications" \
    "$stage/usr/share/icons/hicolor/scalable/apps" \
    "$stage/usr/share/bash-completion/completions" \
    "$stage/usr/share/zsh/site-functions" \
    "$stage/usr/share/fish/vendor_completions.d" \
    "$stage/usr/share/man/man1" \
    "$stage/usr/share/licenses/huskmap" \
    "$stage/usr/share/doc/huskmap"

  install -m 0755 "$BINARY" "$stage/usr/bin/huskmap"
  install -m 0644 "$ROOT_DIR/packaging/$APP_ID.desktop" "$stage/usr/share/applications/$APP_ID.desktop"
  local size
  for size in 32 48 64 128 256 512; do
    install -d "$stage/usr/share/icons/hicolor/${size}x${size}/apps"
    install -m 0644 "$ROOT_DIR/packaging/icons/huskmap-$size.png" \
      "$stage/usr/share/icons/hicolor/${size}x${size}/apps/huskmap.png"
  done
  install -m 0644 "$ROOT_DIR/packaging/icons/huskmap.svg" \
    "$stage/usr/share/icons/hicolor/scalable/apps/huskmap.svg"
  HUSKMAP_LANG=en "$BINARY" completions bash > "$stage/usr/share/bash-completion/completions/huskmap"
  HUSKMAP_LANG=en "$BINARY" completions zsh > "$stage/usr/share/zsh/site-functions/_huskmap"
  HUSKMAP_LANG=en "$BINARY" completions fish > "$stage/usr/share/fish/vendor_completions.d/huskmap.fish"
  HUSKMAP_LANG=en "$BINARY" man | gzip -9n > "$stage/usr/share/man/man1/huskmap.1.gz"
  install -m 0644 "$ROOT_DIR/LICENSE" "$stage/usr/share/licenses/huskmap/LICENSE"
  install -m 0644 "$ROOT_DIR/README.md" "$stage/usr/share/doc/huskmap/README.md"
  install -m 0644 "$ROOT_DIR/huskmap-gui/assets/brands/SOURCES.md" "$stage/usr/share/doc/huskmap/BRANDS.md"
  chmod 0644 "$stage"/usr/share/bash-completion/completions/huskmap \
    "$stage"/usr/share/zsh/site-functions/_huskmap \
    "$stage"/usr/share/fish/vendor_completions.d/huskmap.fish \
    "$stage"/usr/share/man/man1/huskmap.1.gz
}

STAGE="$TMP_DIR/root"
populate_root "$STAGE"
INSTALLED_KB="$(du -sk "$STAGE" | awk '{print $1}')"
INSTALLED_BYTES="$(du -sb "$STAGE" | awk '{print $1}')"

# --- Debian -----------------------------------------------------------------
DEB_ROOT="$TMP_DIR/deb"
cp -a "$STAGE" "$DEB_ROOT"
install -d "$DEB_ROOT/DEBIAN"
cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: huskmap
Version: $VERSION
Section: devel
Priority: optional
Architecture: $DEB_ARCH
Maintainer: $MAINTAINER
Installed-Size: $INSTALLED_KB
Homepage: $URL
Depends: libc6, libgcc-s1, libstdc++6, libfontconfig1, libfreetype6, libegl1, libgl1, libx11-6, libxkbcommon0, libwayland-client0 | libxkbcommon-x11-0
Recommends: git, xdg-utils, libxkbcommon-x11-0, libxcursor1, libxrandr2, libxi6
Description: $SUMMARY
 huskmap reads agent homes (Claude Code, Codex, Cursor, OpenCode, Aider,
 Gemini, Grok), git worktree topology and package caches, and shows what the
 agents left on disk. Worktrees someone is working in, or holding commits that
 exist nowhere else, are guarded. Nothing moves until you apply a plan, and
 then it goes to the trash.
EOF
dpkg-deb --build --root-owner-group "$DEB_ROOT" "$OUTPUT_DIR/huskmap_${VERSION}_${DEB_ARCH}.deb" >/dev/null
echo "created: $OUTPUT_DIR/huskmap_${VERSION}_${DEB_ARCH}.deb"

# --- RPM --------------------------------------------------------------------
if command -v rpmbuild >/dev/null 2>&1; then
  RPM_TOP="$TMP_DIR/rpm"
  mkdir -p "$RPM_TOP"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  cp -a "$STAGE" "$TMP_DIR/huskmap-$VERSION"
  tar -C "$TMP_DIR" -czf "$RPM_TOP/SOURCES/huskmap-$VERSION.tar.gz" "huskmap-$VERSION"
  cat > "$RPM_TOP/SPECS/huskmap.spec" <<EOF
Name:           huskmap
Version:        ${VERSION//-/~}
Release:        1%{?dist}
Summary:        $SUMMARY
License:        MIT
URL:            $URL
Source0:        huskmap-$VERSION.tar.gz
BuildArch:      $NATIVE_ARCH
AutoReqProv:    no
Requires:       glibc, libgcc, libstdc++, fontconfig, freetype
Requires:       (libglvnd-egl or mesa-libEGL)
Requires:       (libglvnd-glx or mesa-libGL)
Requires:       libX11, libxkbcommon
Recommends:     git, xdg-utils, libxkbcommon-x11, libXcursor, libXrandr, libXi

%description
huskmap reads agent homes, git worktree topology and package caches, and
shows what coding agents left on disk. Occupied or unsaved worktrees are
guarded. Nothing moves until you apply a plan, and then it goes to the trash.

%prep
%setup -q -n huskmap-$VERSION

%install
rm -rf %{buildroot}
cp -a . %{buildroot}/

%files
/usr/bin/huskmap
/usr/share/applications/$APP_ID.desktop
/usr/share/icons/hicolor/*/apps/huskmap.*
/usr/share/bash-completion/completions/huskmap
/usr/share/zsh/site-functions/_huskmap
/usr/share/fish/vendor_completions.d/huskmap.fish
/usr/share/man/man1/huskmap.1.gz
/usr/share/licenses/huskmap/LICENSE
/usr/share/doc/huskmap
EOF
  rpmbuild --define "_topdir $RPM_TOP" --target "$NATIVE_ARCH" -bb "$RPM_TOP/SPECS/huskmap.spec" >/dev/null
  RPM_FILE="$(find "$RPM_TOP/RPMS" -type f -name '*.rpm' -print -quit)"
  [[ -n "$RPM_FILE" ]] || { echo "rpmbuild did not produce an RPM" >&2; exit 1; }
  cp "$RPM_FILE" "$OUTPUT_DIR/huskmap-${VERSION}-1.${NATIVE_ARCH}.rpm"
  echo "created: $OUTPUT_DIR/huskmap-${VERSION}-1.${NATIVE_ARCH}.rpm"
elif [[ "${HUSKMAP_SKIP_RPM:-0}" == 1 ]]; then
  echo "skipped: rpm (rpmbuild not installed)"
else
  echo "rpmbuild is required (or set HUSKMAP_SKIP_RPM=1)" >&2
  exit 1
fi

# --- Arch and Alpine --------------------------------------------------------
command -v zstd >/dev/null || { echo "zstd is required for the Arch package" >&2; exit 1; }
BUILDDATE="${SOURCE_DATE_EPOCH:-$(date -u +%s)}"

ARCH_STAGE="$TMP_DIR/arch"
cp -a "$STAGE" "$ARCH_STAGE"
cat > "$ARCH_STAGE/.PKGINFO" <<EOF
pkgname = huskmap
pkgbase = huskmap
pkgver = ${VERSION//-/_}-1
pkgdesc = $SUMMARY
url = $URL
builddate = $BUILDDATE
packager = $MAINTAINER
size = $INSTALLED_BYTES
arch = $NATIVE_ARCH
license = MIT
depend = glibc
depend = gcc-libs
depend = fontconfig
depend = freetype2
depend = libglvnd
depend = libx11
depend = libxkbcommon
optdepend = git: worktree facts
optdepend = xdg-utils: open folders from the map
optdepend = wayland: Wayland sessions
EOF
ARCH_OUT="$OUTPUT_DIR/huskmap-${VERSION}-1-${NATIVE_ARCH}.pkg.tar.zst"
(
  cd "$ARCH_STAGE"
  tar --format=gnu --owner=0 --group=0 --numeric-owner -cf - .PKGINFO usr
) | zstd -q -T0 -19 -c > "$ARCH_OUT"
echo "created: $ARCH_OUT"

APK_STAGE="$TMP_DIR/apk"
cp -a "$STAGE" "$APK_STAGE"
cat > "$APK_STAGE/.PKGINFO" <<EOF
pkgname = huskmap
pkgver = ${VERSION}-r0
pkgdesc = $SUMMARY
url = $URL
builddate = $BUILDDATE
size = $INSTALLED_BYTES
arch = $NATIVE_ARCH
origin = huskmap
maintainer = $MAINTAINER
license = MIT
depend = gcompat
depend = libstdc++
depend = fontconfig
depend = freetype
depend = mesa-egl
depend = mesa-gl
depend = libx11
depend = libxkbcommon
EOF
APK_OUT="$OUTPUT_DIR/huskmap-${VERSION}-r0-${NATIVE_ARCH}.apk"
(
  cd "$APK_STAGE"
  tar --format=ustar --owner=0 --group=0 --numeric-owner -czf "$APK_OUT" .PKGINFO usr
)
echo "created: $APK_OUT"

# --- Portable ---------------------------------------------------------------
PORTABLE="$TMP_DIR/portable/huskmap"
mkdir -p "$PORTABLE"
cp -a "$STAGE/usr/." "$PORTABLE/"
install -m 0755 "$ROOT_DIR/packaging/install-portable.sh" "$PORTABLE/install.sh"
TAR_OUT="$OUTPUT_DIR/huskmap-linux-${TAR_ARCH}.tar.gz"
tar -C "$TMP_DIR/portable" --owner=0 --group=0 --numeric-owner -czf "$TAR_OUT" huskmap
echo "created: $TAR_OUT"

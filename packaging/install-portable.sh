#!/usr/bin/env bash
# Install the portable huskmap tree. Run from inside the unpacked archive.
#   ./install.sh            -> /usr/local (needs root or sudo)
#   ./install.sh --user     -> ~/.local, no root
#   ./install.sh --prefix P -> anywhere
#   ./install.sh --uninstall [--user|--prefix P]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX=/usr/local
UNINSTALL=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --user) PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"; PREFIX="${PREFIX%/share}"; shift ;;
    --prefix) PREFIX="${2:?}"; shift 2 ;;
    --uninstall) UNINSTALL=1; shift ;;
    -h|--help) sed -n '2,6p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# Writable if the prefix, or the first ancestor that exists, is writable.
writable_prefix() {
  local p="$PREFIX"
  while [[ ! -e "$p" && "$p" != / ]]; do p="$(dirname "$p")"; done
  [[ -w "$p" ]]
}

as_needed() {
  if writable_prefix; then
    "$@"
  elif [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    echo "no write access to $PREFIX; try --user" >&2
    exit 1
  fi
}

FILES=(
  bin/huskmap
  share/applications/lucas.cavalheri.huskmap.desktop
  share/bash-completion/completions/huskmap
  share/zsh/site-functions/_huskmap
  share/fish/vendor_completions.d/huskmap.fish
  share/man/man1/huskmap.1.gz
)

if [[ "$UNINSTALL" == 1 ]]; then
  for f in "${FILES[@]}"; do as_needed rm -f "$PREFIX/$f"; done
  as_needed find "$PREFIX/share/icons/hicolor" -name 'huskmap.*' -delete 2>/dev/null || true
  as_needed rm -rf "$PREFIX/share/licenses/huskmap" "$PREFIX/share/doc/huskmap"
  echo "removed huskmap from $PREFIX"
  exit 0
fi

as_needed mkdir -p "$PREFIX"
( cd "$HERE" && find bin share -type f ) | while IFS= read -r f; do
  mode=0644
  [[ "$f" == bin/* ]] && mode=0755
  as_needed install -D -m "$mode" "$HERE/$f" "$PREFIX/$f"
done
command -v gtk-update-icon-cache >/dev/null 2>&1 && as_needed gtk-update-icon-cache -q "$PREFIX/share/icons/hicolor" 2>/dev/null || true
command -v update-desktop-database >/dev/null 2>&1 && as_needed update-desktop-database -q "$PREFIX/share/applications" 2>/dev/null || true
echo "installed huskmap to $PREFIX/bin/huskmap"
case ":$PATH:" in
  *":$PREFIX/bin:"*) ;;
  *) echo "note: $PREFIX/bin is not on PATH" ;;
esac

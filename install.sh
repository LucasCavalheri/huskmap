#!/usr/bin/env bash
# huskmap installer and updater for Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/LucasCavalheri/huskmap/main/install.sh | bash
#
# Picks the release asset for this chip and package manager, checks it against the
# release's SHA256SUMS, and installs it. When huskmap is already open it says what is in
# flight: a running apply is waited for (never interrupted), marks are kept for the next
# window, and the map reopens after the update.
set -euo pipefail

REPO="LucasCavalheri/huskmap"
RELEASES="https://github.com/${REPO}/releases"
APP_BIN="huskmap"
APPLY_WAIT="${HUSKMAP_APPLY_WAIT:-900}"

usage() {
  cat <<'EOF'
usage: install.sh [--print-plan] [--version VERSION] [--user] [--yes]

  --print-plan   print arch, family, asset and URLs, then exit
  --version VER  install this tag (default: latest release)
  --user         portable install into ~/.local, no root
  --yes          do not ask before closing an open huskmap
EOF
}

PRINT_PLAN=0
USER_INSTALL=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --print-plan) PRINT_PLAN=1; shift ;;
    --version) HUSKMAP_VERSION="${2:?}"; shift 2 ;;
    --user) USER_INSTALL=1; shift ;;
    --yes|-y) HUSKMAP_AUTO_CONFIRM=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

say() { printf '  %s\n' "$*" >&2; }

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "install.sh needs $1 on PATH" >&2; exit 1; }
}

normalize_arch() {
  case "$1" in
    x86_64 | amd64) echo x86_64 ;;
    aarch64 | arm64) echo aarch64 ;;
    *) echo "huskmap ships for x86_64 and aarch64, not: $1" >&2; return 1 ;;
  esac
}

detect_arch() { normalize_arch "${HUSKMAP_ARCH:-$(uname -m)}"; }

detect_pm() {
  if [[ -n "${HUSKMAP_PM:-}" ]]; then echo "$HUSKMAP_PM"; return; fi
  if (( USER_INSTALL == 1 )); then echo user; return; fi
  if command -v pacman >/dev/null 2>&1; then echo pacman
  elif command -v apk >/dev/null 2>&1; then echo apk
  elif command -v apt-get >/dev/null 2>&1; then echo apt
  elif command -v dnf >/dev/null 2>&1; then echo dnf
  elif command -v yum >/dev/null 2>&1; then echo yum
  elif command -v zypper >/dev/null 2>&1; then echo zypper
  else echo tar
  fi
}

latest_version() {
  if [[ -n "${HUSKMAP_VERSION:-}" ]]; then echo "${HUSKMAP_VERSION#v}"; return; fi
  need curl
  local url tag
  url="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "${RELEASES}/latest")"
  tag="${url%/}"
  tag="${tag##*/}"
  [[ "$tag" != latest ]] || { echo "no huskmap release is published yet" >&2; exit 1; }
  echo "${tag#v}"
}

asset_for() {
  local arch="$1" pm="$2" ver="$3"
  local deb=amd64 tar=x64
  [[ "$arch" == aarch64 ]] && deb=arm64 && tar=arm64
  case "$pm" in
    apt) echo "huskmap_${ver}_${deb}.deb" ;;
    dnf | yum | zypper) echo "huskmap-${ver}-1.${arch}.rpm" ;;
    pacman) echo "huskmap-${ver}-1-${arch}.pkg.tar.zst" ;;
    apk) echo "huskmap-${ver}-r0-${arch}.apk" ;;
    tar | user) echo "huskmap-linux-${tar}.tar.gz" ;;
    *) echo "unknown package family: $pm" >&2; return 1 ;;
  esac
}

run_root() {
  if [[ "$(id -u)" -eq 0 ]]; then "$@"
  elif command -v sudo >/dev/null 2>&1; then sudo "$@"
  else echo "install.sh needs root or sudo (or use --user)" >&2; exit 1
  fi
}

# --- what the running huskmap is doing ---------------------------------------

# `huskmap status` prints key=value lines. Older or missing binaries print nothing.
status_value() {
  local key="$1"
  if [[ -n "${HUSKMAP_STATUS:-}" ]]; then
    printf '%s\n' "$HUSKMAP_STATUS" | sed -n "s/^${key}=//p" | head -n 1
    return
  fi
  command -v "$APP_BIN" >/dev/null 2>&1 || return 0
  "$APP_BIN" status 2>/dev/null | sed -n "s/^${key}=//p" | head -n 1
}

running_pids() {
  if [[ -n "${HUSKMAP_RUNNING_PIDS+x}" ]]; then
    printf '%s\n' $HUSKMAP_RUNNING_PIDS
    return
  fi
  if command -v pgrep >/dev/null 2>&1; then
    pgrep -u "$(id -u)" -x "$APP_BIN" || true
  fi
}

pid_alive() {
  local stat state
  [[ -r "/proc/$1/stat" ]] || return 1
  stat="$(<"/proc/$1/stat")"
  state="${stat##*) }"
  state="${state%% *}"
  [[ "$state" != Z ]]
}

# Never interrupt an apply: husks are half in the trash while it runs.
wait_for_apply() {
  local waited=0 husks
  while [[ "$(status_value applying)" == 1 ]]; do
    if (( waited == 0 )); then
      husks="$(status_value apply_husks)"
      say "huskmap is sending ${husks:-some} husks to the trash. Waiting for it to finish."
    fi
    if (( waited >= APPLY_WAIT )); then
      echo "an apply is still running after ${APPLY_WAIT}s; try again when it ends" >&2
      return 1
    fi
    sleep "${HUSKMAP_POLL:-2}"
    waited=$(( waited + ${HUSKMAP_POLL:-2} ))
  done
}

confirm_close() {
  local phase marked human answer
  phase="$(status_value phase)"
  marked="$(status_value marked)"
  human="$(status_value marked_human)"
  say "huskmap is open${phase:+ (${phase})}."
  if [[ -n "$marked" && "$marked" != 0 ]]; then
    say "${marked} marked husks (${human}) stay marked; the map reopens with them."
  fi
  [[ "$phase" == scanning ]] && say "The scan in progress stops; it only reads, nothing is lost."
  say "Updating closes the window and reopens it after."
  [[ "${HUSKMAP_AUTO_CONFIRM:-0}" == 1 ]] && return 0
  if ! (: </dev/tty) 2>/dev/null; then
    echo "no terminal to confirm on; close huskmap or pass --yes" >&2
    return 1
  fi
  read -r -p "  Continue? [y/N] " answer < /dev/tty
  [[ "$answer" =~ ^([Yy]([Ee][Ss])?|[Ss]([Ii][Mm])?)$ ]] || { echo "cancelled" >&2; return 1; }
}

stop_app() {
  local pid
  for pid in "$@"; do kill -TERM "$pid" 2>/dev/null || true; done
  for _ in $(seq 1 50); do
    local alive=0
    for pid in "$@"; do pid_alive "$pid" && alive=1; done
    (( alive == 0 )) && return 0
    sleep 0.1
  done
  for pid in "$@"; do pid_alive "$pid" && kill -KILL "$pid" 2>/dev/null || true; done
}

relaunch() {
  [[ -n "${WAYLAND_DISPLAY:-}${DISPLAY:-}" ]] || { say "no display here; run huskmap gui later"; return 0; }
  local launcher
  launcher="$(command -v "$APP_BIN" || true)"
  [[ -n "$launcher" ]] || { say "huskmap is not on PATH; launch it from the menu"; return 0; }
  nohup "$launcher" gui >/dev/null 2>&1 </dev/null &
  say "reopened the map"
}

# --- install ------------------------------------------------------------------

verify() {
  local file="$1" sums="$2" name expected actual
  name="$(basename "$file")"
  expected="$(awk -v n="$name" '$2 == n || $2 == "*" n { print $1 }' "$sums" | head -n 1)"
  [[ -n "$expected" ]] || { echo "SHA256SUMS does not list $name; refusing" >&2; return 1; }
  actual="$(sha256sum "$file" | awk '{print $1}')"
  [[ "$actual" == "$expected" ]] || { echo "checksum mismatch for $name; refusing" >&2; return 1; }
}

install_asset() {
  local pm="$1" file="$2"
  case "$pm" in
    apt) run_root apt-get install -y "$file" ;;
    dnf) run_root dnf install -y "$file" ;;
    yum) run_root yum install -y "$file" ;;
    zypper) run_root zypper --non-interactive install --allow-unsigned-rpm "$file" ;;
    pacman) run_root pacman -U --noconfirm "$file" ;;
    apk) run_root apk add gcompat && run_root apk add --allow-untrusted "$file" ;;
    tar | user)
      local dir
      dir="$(mktemp -d)"
      tar -xzf "$file" -C "$dir"
      if [[ "$pm" == user ]]; then
        "$dir/huskmap/install.sh" --user
      else
        "$dir/huskmap/install.sh"
      fi
      rm -rf "$dir"
      ;;
    *) echo "cannot install family $pm" >&2; return 1 ;;
  esac
}

main() {
  local arch pm version asset url sums_url
  arch="$(detect_arch)"
  pm="$(detect_pm)"
  version="$(latest_version)"
  asset="$(asset_for "$arch" "$pm" "$version")"
  url="${RELEASES}/download/v${version}/${asset}"
  sums_url="${RELEASES}/download/v${version}/SHA256SUMS"

  if (( PRINT_PLAN == 1 )); then
    printf 'arch=%s\nfamily=%s\nversion=%s\nasset=%s\nurl=%s\nsums=%s\n' \
      "$arch" "$pm" "$version" "$asset" "$url" "$sums_url"
    return 0
  fi

  need curl
  need sha256sum
  echo "huskmap ${version} · ${arch} · ${pm}" >&2
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT
  local tmp="$TMP_DIR"
  say "downloading ${asset}"
  curl -fL --progress-bar -o "$tmp/$asset" "$url"
  curl -fsSL -o "$tmp/SHA256SUMS" "$sums_url"
  verify "$tmp/$asset" "$tmp/SHA256SUMS"
  say "checksum ok"
  # apt reads local files as _apt; mktemp dirs are 0700.
  chmod 0755 "$tmp"
  chmod 0644 "$tmp/$asset"

  local pids was_open=0
  pids="$(running_pids | tr '\n' ' ')"
  wait_for_apply
  if [[ -n "${pids// /}" ]]; then
    confirm_close
    was_open=1
    wait_for_apply
    # shellcheck disable=SC2086
    stop_app $pids
  fi
  install_asset "$pm" "$tmp/$asset"
  (( was_open == 1 )) && relaunch
  echo "done. huskmap ${version} is installed: run huskmap, or open it from the menu." >&2
}

if [[ "${HUSKMAP_SOURCE_ONLY:-0}" != 1 ]]; then
  main "$@"
fi

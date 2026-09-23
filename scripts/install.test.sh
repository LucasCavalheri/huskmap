#!/usr/bin/env bash
# Exercise install.sh without touching the host: plans, checksums, apply waits, the
# running-window warning, and a real portable install into a throwaway HOME.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
pass=0
fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { pass=$((pass + 1)); }

plan() { HUSKMAP_VERSION=0.2.0 HUSKMAP_ARCH="$1" HUSKMAP_PM="$2" bash "$ROOT/install.sh" --print-plan; }

# --- plans for every family ---
check_plan() {
  local out
  out="$(plan "$1" "$2")"
  grep -qx "asset=$3" <<<"$out" || fail "plan $1/$2: $out"
  grep -qx "url=https://github.com/LucasCavalheri/huskmap/releases/download/v0.2.0/$3" <<<"$out" || fail "url $1/$2"
  grep -qx "sums=https://github.com/LucasCavalheri/huskmap/releases/download/v0.2.0/SHA256SUMS" <<<"$out" || fail "sums $1/$2"
  ok
}
check_plan x86_64 apt huskmap_0.2.0_amd64.deb
check_plan arm64 apt huskmap_0.2.0_arm64.deb
check_plan amd64 dnf huskmap-0.2.0-1.x86_64.rpm
check_plan aarch64 zypper huskmap-0.2.0-1.aarch64.rpm
check_plan x86_64 pacman huskmap-0.2.0-1-x86_64.pkg.tar.zst
check_plan x86_64 apk huskmap-0.2.0-r0-x86_64.apk
check_plan x86_64 tar huskmap-linux-x64.tar.gz
check_plan aarch64 tar huskmap-linux-arm64.tar.gz
out="$(HUSKMAP_VERSION=v0.2.0 HUSKMAP_ARCH=x86_64 bash "$ROOT/install.sh" --print-plan --user)"
grep -qx "family=user" <<<"$out" || fail "--user family"; ok
grep -qx "version=0.2.0" <<<"$out" || fail "v prefix stripped"; ok
if HUSKMAP_VERSION=0.2.0 HUSKMAP_ARCH=riscv64 bash "$ROOT/install.sh" --print-plan 2>/dev/null; then
  fail "riscv64 accepted"
fi; ok
if bash "$ROOT/install.sh" --bogus 2>/dev/null; then fail "bogus flag accepted"; fi; ok

# --- sourced helpers ---
export HUSKMAP_SOURCE_ONLY=1
# shellcheck source=/dev/null
source "$ROOT/install.sh"

echo "payload" > "$WORK/huskmap-linux-x64.tar.gz"
( cd "$WORK" && sha256sum huskmap-linux-x64.tar.gz > SHA256SUMS )
verify "$WORK/huskmap-linux-x64.tar.gz" "$WORK/SHA256SUMS" || fail "good checksum refused"; ok
echo "tampered" > "$WORK/huskmap-linux-x64.tar.gz"
if verify "$WORK/huskmap-linux-x64.tar.gz" "$WORK/SHA256SUMS" 2>/dev/null; then fail "tampered accepted"; fi; ok
: > "$WORK/EMPTY"
if verify "$WORK/huskmap-linux-x64.tar.gz" "$WORK/EMPTY" 2>/dev/null; then fail "unlisted accepted"; fi; ok

# a fake huskmap whose apply finishes after two polls
mkdir -p "$WORK/bin"
cat > "$WORK/bin/huskmap" <<FAKE
#!/usr/bin/env bash
n=\$(cat "$WORK/calls" 2>/dev/null || echo 0); n=\$((n + 1)); echo "\$n" > "$WORK/calls"
if [[ "\$1" == status ]]; then
  if (( n <= 4 )); then echo applying=1; echo apply_husks=7; else echo applying=0; fi
  echo phase=ready; echo marked=4; echo marked_human=1.2 GB
fi
FAKE
chmod +x "$WORK/bin/huskmap"
PATH="$WORK/bin:$PATH" HUSKMAP_POLL=0 wait_for_apply 2>"$WORK/wait.log" || fail "apply wait failed"
grep -q "sending 7 husks" "$WORK/wait.log" || fail "apply wait message"; ok
rm -f "$WORK/calls"
if PATH="$WORK/bin:$PATH" HUSKMAP_POLL=1 APPLY_WAIT=0 wait_for_apply 2>/dev/null; then
  fail "apply wait must give up at the limit, never kill"
fi; ok

msg="$(HUSKMAP_STATUS=$'phase=scanning\nmarked=4\nmarked_human=1.2 GB' HUSKMAP_AUTO_CONFIRM=1 confirm_close 2>&1)"
grep -q "4 marked husks (1.2 GB) stay marked" <<<"$msg" || fail "marks warning: $msg"; ok
grep -q "only reads" <<<"$msg" || fail "scan warning"; ok
msg="$(HUSKMAP_STATUS=$'phase=ready\nmarked=0' HUSKMAP_AUTO_CONFIRM=1 confirm_close 2>&1)"
grep -q "stay marked" <<<"$msg" && fail "no marks, no marks line"; ok
if ! (: </dev/tty) 2>/dev/null; then
  if HUSKMAP_STATUS="phase=ready" HUSKMAP_AUTO_CONFIRM=0 confirm_close 2>/dev/null; then
    fail "without a terminal and without --yes it must refuse"
  fi
fi; ok

[[ "$(HUSKMAP_RUNNING_PIDS="11 12" running_pids | tr '\n' ' ')" == "11 12 " ]] || fail "pids override"; ok
pid_alive $$ || fail "self is alive"; ok
pid_alive 999999999 && fail "impossible pid alive"; ok

# --- a real portable install into a throwaway HOME ---
if [[ -n "${HUSKMAP_PORTABLE_TAR:-}" && -f "$HUSKMAP_PORTABLE_TAR" ]]; then
  cp "$HUSKMAP_PORTABLE_TAR" "$WORK/huskmap-linux-x64.tar.gz"
else
  mkdir -p "$WORK/tree/huskmap/bin" "$WORK/tree/huskmap/share/applications"
  printf '#!/bin/sh\necho huskmap 9.9.9\n' > "$WORK/tree/huskmap/bin/huskmap"
  chmod +x "$WORK/tree/huskmap/bin/huskmap"
  cp "$ROOT/packaging/lucas.cavalheri.huskmap.desktop" "$WORK/tree/huskmap/share/applications/"
  install -m 0755 "$ROOT/packaging/install-portable.sh" "$WORK/tree/huskmap/install.sh"
  tar -C "$WORK/tree" -czf "$WORK/huskmap-linux-x64.tar.gz" huskmap
fi
HOME="$WORK/home" XDG_DATA_HOME="$WORK/home/.local/share" install_asset user "$WORK/huskmap-linux-x64.tar.gz" >/dev/null
[[ -x "$WORK/home/.local/bin/huskmap" ]] || fail "portable binary not installed"; ok
[[ -f "$WORK/home/.local/share/applications/lucas.cavalheri.huskmap.desktop" ]] || fail "desktop file"; ok
HOME="$WORK/home" XDG_DATA_HOME="$WORK/home/.local/share" "$ROOT/packaging/install-portable.sh" --user --uninstall >/dev/null
[[ ! -e "$WORK/home/.local/bin/huskmap" ]] || fail "uninstall left the binary"; ok

echo "install.sh: $pass checks passed"

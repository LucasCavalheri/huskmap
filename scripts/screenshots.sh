#!/usr/bin/env bash
# Every GUI frame at every window size, from the synthetic report (never a real home).
#
#   scripts/screenshots.sh [--sizes all|1366x768,1920x1080] [--video] [--out DIR]
#
# Sizes default to all six in huskmap-gui/tests/snapshots.rs (minimum 1120x720 up to 2560x1440).
# Writes NAME.png (default size) and NAME@WxH.png per size into DIR (default target/tmp/snapshots).
# --video also records the motion clips (map reveal, drawer, scan pulse): clips/NAME.mp4 to watch,
# and clips/NAME-strip.png, 12 frames in one image, for an agent that can only read images.
set -euo pipefail
cd "$(dirname "$0")/.."

sizes=all
video=0
out=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --sizes) sizes="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --video) video=1; shift ;;
    -h|--help) sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

frames=target/tmp/snapshots
rm -rf "$frames"/*.png "$frames"/clips
tests=(frames ledger_scrolls)
[[ $video == 1 ]] && tests+=(motion_clips)
HUSKMAP_SNAPSHOT_SIZES="$sizes" HUSKMAP_SNAPSHOT_CLIPS="$video" \
  cargo test -q -p huskmap-gui --features desktop --test snapshots -- "${tests[@]}" >&2

if [[ $video == 1 ]]; then
  command -v ffmpeg >/dev/null || { echo "--video needs ffmpeg" >&2; exit 1; }
  for clip in "$frames"/clips/*/; do
    name=$(basename "$clip")
    ffmpeg -loglevel error -y -framerate 30 -i "$clip/%04d.png" \
      -vf "pad=ceil(iw/2)*2:ceil(ih/2)*2" -c:v libx264 -pix_fmt yuv420p "$frames/clips/$name.mp4"
    total=$(find "$clip" -name '*.png' | wc -l)
    every=$(( total / 12 > 0 ? total / 12 : 1 ))
    ffmpeg -loglevel error -y -i "$clip/%04d.png" \
      -vf "select='not(mod(n\,$every))',scale=640:-1,tile=4x3:padding=4:color=black" \
      -frames:v 1 -update 1 "$frames/clips/$name-strip.png"
    rm -rf "$clip"
  done
fi

if [[ -n "$out" ]]; then
  mkdir -p "$out"
  cp -r "$frames"/*.png "$out"/
  [[ -d "$frames/clips" ]] && cp -r "$frames/clips" "$out"/
  frames="$out"
fi
count=$(find "$frames" -maxdepth 1 -name '*.png' | wc -l)
echo "$count frames in $frames"

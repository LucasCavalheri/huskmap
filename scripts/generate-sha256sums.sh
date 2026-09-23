#!/usr/bin/env bash
# Write SHA256SUMS for every file in a release directory (the updater refuses without it).
set -euo pipefail
DIR="${1:?release dir}"
cd "$DIR"
rm -f SHA256SUMS
find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%f\n' | LC_ALL=C sort | xargs -r sha256sum > SHA256SUMS
[[ -s SHA256SUMS ]] || { echo "no files to sum in $DIR" >&2; exit 1; }
cat SHA256SUMS

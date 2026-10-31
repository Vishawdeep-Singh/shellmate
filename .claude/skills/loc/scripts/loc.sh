#!/usr/bin/env bash
# Usage: loc.sh [DIR] [EXT] [TOP]
# Counts lines per file with extension EXT under DIR and prints the TOP largest.
set -euo pipefail

dir="${1:-.}"
ext="${2:-rs}"
top="${3:-10}"

if [ ! -d "$dir" ]; then
  echo "error: '$dir' is not a directory" >&2
  exit 1
fi

case "$top" in
  '' | *[!0-9]*)
    echo "error: TOP must be a number, got '$top'" >&2
    exit 1
    ;;
esac

counts=$(
  find "$dir" -type f -name "*.$ext" -not -path '*/target/*' -not -path '*/.git/*' -print0 |
    xargs -0 wc -l |
    grep -v ' total$' |
    sort -rn
) || true

if [ -z "$counts" ]; then
  echo "no .$ext files under $dir"
  exit 0
fi

files=$(printf '%s\n' "$counts" | wc -l | tr -d ' ')
total=$(printf '%s\n' "$counts" | awk '{ sum += $1 } END { print sum }')

echo ".$ext files under $dir: $files files, $total lines"
echo "top $top:"
printf '%s\n' "$counts" | awk -v n="$top" 'NR <= n { printf "  %6d  %s\n", $1, $2 }'

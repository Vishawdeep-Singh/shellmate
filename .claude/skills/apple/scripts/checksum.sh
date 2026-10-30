#!/bin/sh
# Prints the first 8 characters of the SHA-256 checksum of the skill's data.txt.
# Works from any directory, on macOS (shasum) and Linux (sha256sum).
set -eu

skill_dir=$(cd "$(dirname "$0")/.." && pwd)
data_file="$skill_dir/data.txt"

if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$data_file" | cut -c1-8
else
    shasum -a 256 "$data_file" | cut -c1-8
fi

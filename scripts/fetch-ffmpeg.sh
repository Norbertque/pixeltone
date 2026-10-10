#!/usr/bin/env bash
# Fetches a prebuilt LGPL shared FFmpeg 8.1 (BtbN/FFmpeg-Builds) into third_party/ffmpeg/<platform>/
# and verifies its SHA-256. Used for Linux development and by CI on the Windows runner, so both
# link the same FFmpeg release branch.
#
#   scripts/fetch-ffmpeg.sh [linux64|win64] [--force]
#
# Layout afterwards: third_party/ffmpeg/<platform>/{bin,include,lib}, plus FETCHED.txt.
set -euo pipefail

BRANCH="n8.1"
SERIES="8.1"
BASE_URL="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest"

platform="${1:-}"
force="${2:-}"
if [[ -z "$platform" ]]; then
  case "$(uname -s)" in
    Linux*) platform="linux64" ;;
    MINGW*|MSYS*|CYGWIN*) platform="win64" ;;
    *) echo "Unsupported host; pass linux64 or win64" >&2; exit 1 ;;
  esac
fi

case "$platform" in
  linux64) ext="tar.xz" ;;
  win64) ext="zip" ;;
  *) echo "Unknown platform '$platform' (expected linux64 or win64)" >&2; exit 1 ;;
esac

asset="ffmpeg-${BRANCH}-latest-${platform}-lgpl-shared-${SERIES}.${ext}"
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/third_party/ffmpeg/$platform"

if [[ -f "$dest/FETCHED.txt" && "$force" != "--force" ]]; then
  echo "FFmpeg already present in $dest (use --force to refetch):"
  cat "$dest/FETCHED.txt"
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "Downloading $asset ..."
curl -fsSL --retry 4 --retry-delay 2 -o "$work/checksums.sha256" "$BASE_URL/checksums.sha256"
curl -fsSL --retry 4 --retry-delay 2 -o "$work/$asset" "$BASE_URL/$asset"

expected="$(grep -E "[ *]${asset}\$" "$work/checksums.sha256" | awk '{print $1}')"
if [[ -z "$expected" ]]; then
  echo "No checksum listed for $asset" >&2
  exit 1
fi
actual="$(sha256sum "$work/$asset" | awk '{print $1}')"
if [[ "$expected" != "$actual" ]]; then
  echo "Checksum mismatch for $asset: expected $expected, got $actual" >&2
  exit 1
fi
echo "Checksum OK ($actual)"

mkdir -p "$work/extract"
if [[ "$ext" == "zip" ]]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "$work/$asset" -d "$work/extract"
  else
    7z x -y -o"$work/extract" "$work/$asset" >/dev/null
  fi
else
  tar -xJf "$work/$asset" -C "$work/extract"
fi

top="$(find "$work/extract" -mindepth 1 -maxdepth 1 -type d | head -n1)"
if [[ -z "$top" || ! -d "$top/include" ]]; then
  echo "Unexpected archive layout in $asset" >&2
  exit 1
fi

rm -rf "$dest"
mkdir -p "$(dirname "$dest")"
mv "$top" "$dest"
{
  echo "asset=$asset"
  echo "sha256=$actual"
  echo "fetched=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$dest/FETCHED.txt"
echo "FFmpeg installed to $dest"

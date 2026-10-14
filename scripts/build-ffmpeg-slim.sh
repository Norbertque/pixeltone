#!/usr/bin/env bash
# Cross-compiles an audio-only, LGPL, shared FFmpeg for Windows x64 into
# third_party/ffmpeg/win64/{bin,include,lib}, the layout scripts/fetch-ffmpeg.sh produces.
#
# The player only decodes audio, so this build keeps the seven libraries the Rust bindings
# link (avcodec, avformat, avutil, swresample, avfilter, avdevice, swscale) but none of the
# video, encoding or device code: about a tenth of the size of a full build. The network
# part is plain HTTP over TCP only, for files a paired phone serves (remote control).
#
#   scripts/build-ffmpeg-slim.sh            # mingw-w64 cross toolchain (CI: Ubuntu)
#   FFMPEG_CC=... FFMPEG_EXTRA_CONFIGURE=... scripts/build-ffmpeg-slim.sh   # other compilers
#
# Needs: curl, tar/xz, make, nasm, and a mingw-w64 GCC (or another Windows-GNU C compiler)
# with dlltool, which turns each DLL's exports into the .lib MSVC links against.
set -euo pipefail

VERSION="8.1.3"
SHA256="7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3"
CROSS_PREFIX="${CROSS_PREFIX-x86_64-w64-mingw32-}"

root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${FFMPEG_DEST:-$root/third_party/ffmpeg/win64}"
work="${FFMPEG_BUILD_DIR:-$(mktemp -d)}"
jobs="$(nproc 2>/dev/null || echo 4)"

# Lossless and lossy audio, DSD (DSF/DFF, DST), the PCM flavours WAV/AIFF/CAF carry.
DECODERS=(
  flac alac als ape tak tta wavpack wmalossless shorten mlp truehd
  mp1 mp1float mp2 mp2float mp3 mp3float aac aac_fixed aac_latm vorbis opus
  wmav1 wmav2 wmapro ac3 ac3_fixed eac3 dca mpc7 mpc8 amrnb amrwb
  adpcm_ima_wav adpcm_ms
  dsd_lsbf dsd_lsbf_planar dsd_msbf dsd_msbf_planar dst
  pcm_alaw pcm_mulaw pcm_f16le pcm_f24le pcm_f32be pcm_f32le pcm_f64be pcm_f64le
  pcm_s8 pcm_s8_planar pcm_u8 pcm_s16be pcm_s16be_planar pcm_s16le pcm_s16le_planar
  pcm_u16be pcm_u16le pcm_s24be pcm_s24le pcm_s24le_planar pcm_u24be pcm_u24le
  pcm_s32be pcm_s32le pcm_s32le_planar pcm_u32be pcm_u32le pcm_s64be pcm_s64le
)
DEMUXERS=(
  flac mov mp3 aac loas ogg matroska wav w64 aiff caf au ape wv tak tta asf xwma
  dsf iff wsd mpc mpc8 ac3 eac3 dts truehd mlp amr shorten
)
PARSERS=(flac mpegaudio aac aac_latm opus vorbis ac3 dca mlp tak amr)

join() {
  local IFS=,
  echo "$*"
}

archive="$work/ffmpeg-$VERSION.tar.xz"
if [[ ! -f "$archive" ]]; then
  echo "Downloading FFmpeg $VERSION ..."
  curl -fsSL --retry 4 --retry-delay 2 -o "$archive" "https://ffmpeg.org/releases/ffmpeg-$VERSION.tar.xz"
fi
echo "$SHA256  $archive" | sha256sum -c -
tar -xf "$archive" -C "$work"
src="$work/ffmpeg-$VERSION"
out="$work/out"

cd "$src"
# Not $CC: that names the host compiler in most environments.
cc_args=()
if [[ -n "${FFMPEG_CC:-}" ]]; then
  cc_args+=(--cc="$FFMPEG_CC")
fi
./configure \
  --prefix="$out" \
  --target-os=mingw32 \
  --arch=x86_64 \
  --enable-cross-compile \
  --cross-prefix="$CROSS_PREFIX" \
  "${cc_args[@]}" \
  --enable-shared \
  --disable-static \
  --disable-programs \
  --disable-doc \
  --disable-debug \
  --disable-autodetect \
  --disable-everything \
  --enable-network \
  --enable-w32threads \
  --enable-protocol=file,http,tcp \
  --enable-decoder="$(join "${DECODERS[@]}")" \
  --enable-demuxer="$(join "${DEMUXERS[@]}")" \
  --enable-parser="$(join "${PARSERS[@]}")" \
  --extra-ldflags="-static-libgcc" \
  ${FFMPEG_EXTRA_CONFIGURE:-}

make -j"$jobs"
make install

# Import libraries for MSVC: configure puts them next to the DLLs or in lib/.
rm -rf "$dest"
mkdir -p "$dest/lib"
cp -r "$out/include" "$dest/include"
mkdir -p "$dest/bin"
cp "$out"/bin/*.dll "$dest/bin/"
find "$out" -name '*.lib' -exec cp {} "$dest/lib/" \;
for lib in avcodec avformat avutil swresample avfilter avdevice swscale; do
  [[ -f "$dest/lib/$lib.lib" ]] || { echo "missing $lib.lib" >&2; exit 1; }
done

# The DLLs may only depend on each other and on Windows itself (no libgcc, no pthreads).
objdump="${OBJDUMP:-${CROSS_PREFIX}objdump}"
if command -v "$objdump" >/dev/null; then
  extra="$("$objdump" -p "$dest"/bin/*.dll | awk '/DLL Name:/ {print tolower($3)}' | sort -u |
    grep -Ev '^(av|sw)[a-z]+-[0-9]+\.dll$|^(kernel32|msvcrt|user32|advapi32|bcrypt|ole32|shell32|ws2_32|api-ms-win-[a-z0-9-]+)\.dll$' || true)"
  if [[ -n "$extra" ]]; then
    echo "Unexpected DLL dependencies: $extra" >&2
    exit 1
  fi
fi

{
  echo "FFmpeg $VERSION, audio-only LGPL shared build (scripts/build-ffmpeg-slim.sh)"
  date -u +"built %Y-%m-%dT%H:%MZ"
} > "$dest/FETCHED.txt"
du -ch "$dest"/bin/*.dll | tail -1

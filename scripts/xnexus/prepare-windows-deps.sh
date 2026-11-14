#!/usr/bin/env bash
# MSYS2 UCRT64; no rustup-default changes and no application launch.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="/ucrt64/bin:$PATH"
PREFIX="$REPO/target/xnexus-vpx-1.17.0"
SHA=1020f184046187baa2985dbde38e0691f49c44088bca7a1842b0236c6081dc0a
mkdir -p "$REPO/target/xnexus-dependencies"
if [ ! -f "$PREFIX/lib/libvpx.a" ] || [ "$(cat "$PREFIX/.xnexus-sha256" 2>/dev/null || true)" != "$SHA" ]; then
  cd "$REPO/target/xnexus-dependencies"
  curl -fL --retry 3 https://github.com/webmproject/libvpx/archive/refs/tags/v1.17.0.tar.gz -o libvpx-1.17.0.tar.gz
  echo "$SHA  libvpx-1.17.0.tar.gz" | sha256sum -c -
  tar xf libvpx-1.17.0.tar.gz
  mkdir -p vpx-build
  cd vpx-build
  CROSS= ../libvpx-1.17.0/configure --target=x86_64-win64-gcc \
    --prefix="$PREFIX" --as=nasm --enable-static --disable-shared \
    --disable-examples --disable-tools --disable-docs --disable-unit-tests \
    --disable-install-docs --disable-install-bins --disable-install-srcs \
    --disable-vp9 --disable-vp8-decoder --enable-vp8-encoder --enable-realtime-only \
    --enable-runtime-cpu-detect --disable-multithread --disable-webm-io --disable-libyuv
  make -j2
  make install
  echo "$SHA" > "$PREFIX/.xnexus-sha256"
fi
bash "$REPO/scripts/fetch-hamlib.sh"
echo "VPX_MINGW_PREFIX=$(cygpath -m "$PREFIX")"

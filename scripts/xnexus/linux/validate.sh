#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in fetch|contract|desktop) phase=$1;; *) echo 'Expected fetch, contract or desktop' >&2; exit 2;; esac
mkdir -p /work/cargo /work/home
export HOME=/work/home CARGO_HOME=/work/cargo CARGO_TARGET_DIR=/work/target
export CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2 RUST_TEST_THREADS=2
export RUSTUP_HOME=/opt/rustup
archive_hash=$(sha256sum /input/source.tar | cut -d' ' -f1)
if [[ -e /work/archive.sha256 ]]; then
  [[ $(cat /work/archive.sha256) == "$archive_hash" ]] || { echo 'Cache belongs to different source archive' >&2; exit 2; }
else
  [[ ! -e /work/source ]] || { echo 'Incomplete source cache; preserve it and use a new cache volume' >&2; exit 2; }
  mkdir /work/source
  tar -xf /input/source.tar -C /work/source
  printf '%s\n' "$archive_hash" > /work/archive.sha256
fi
cd /work/source
cp /opt/packages.tsv /evidence/packages.tsv
{ uname -a; rustc -Vv; cargo -V; node -v; npm -v; gfortran --version; cmake --version; } > /evidence/toolchain.txt
if [[ $phase == fetch ]]; then
  cargo fetch --locked --manifest-path integrations/xlog/Cargo.toml --target x86_64-unknown-linux-gnu
  cargo fetch --locked --manifest-path src-tauri/Cargo.toml --target x86_64-unknown-linux-gnu
  npm --prefix ui ci --ignore-scripts --no-audit --no-fund
elif [[ $phase == contract ]]; then
  # Full upstream gate inventory is recorded, not falsely claimed as executed.
  node scripts/gates --list > /evidence/upstream-gates.txt
  node --test scripts/xnexus/fork-policy.test.mjs
  cargo test --locked --offline --manifest-path integrations/xlog/Cargo.toml
  npm --prefix ui rebuild --offline
  npm --prefix ui run build
  npm --prefix ui test -- src/xlog --maxWorkers=2
else
  cargo check --locked --offline --manifest-path src-tauri/Cargo.toml --features radio --all-targets
fi
printf '%s passed (scoped Linux validation; no package/runtime qualification)\n' "$phase"

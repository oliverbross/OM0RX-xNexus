#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in fetch|contract|desktop|seed) phase=$1;; *) echo 'Expected fetch, contract, desktop or seed' >&2; exit 2;; esac
if [[ $phase == seed ]]; then
  [[ ${XNEXUS_RESOURCE_PROFILE:-} == contract-reduced || ${XNEXUS_RESOURCE_PROFILE:-} == contract-low-headroom ]] || exit 2
  exec python3 /tools/seed_cache.py
fi
[[ ! -e /work/seed.pending.json ]] || { echo 'Incomplete cache seed; preserve it and use a new volume' >&2; exit 2; }
mkdir -p /work/cargo /work/home
export HOME=/work/home CARGO_HOME=/work/cargo CARGO_TARGET_DIR=/work/target
case "${XNEXUS_RESOURCE_PROFILE:-standard}" in
  standard) jobs=2 ;;
  desktop-reduced|desktop-low-headroom)
    [[ $phase == desktop ]] || { echo 'Reduced profile requires desktop phase' >&2; exit 2; }
    jobs=1 ;;
  contract-reduced|contract-low-headroom)
    [[ $phase == contract ]] || { echo 'Reduced contract profile requires contract phase' >&2; exit 2; }
    jobs=1 ;;
  *) echo 'Unknown resource profile' >&2; exit 2 ;;
esac
export CARGO_BUILD_JOBS=$jobs CMAKE_BUILD_PARALLEL_LEVEL=$jobs RUST_TEST_THREADS=$jobs
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
python3 /tools/verify_source.py /input/source.tar /work/source
verify_after_phase() {
  phase_status=$?
  trap - EXIT
  if ! python3 /tools/verify_source.py /input/source.tar /work/source; then
    echo 'Tracked source changed during validation; cached evidence preserved, validation rejected' >&2
    exit 3
  fi
  if [[ $phase_status == 0 ]]; then
    printf '%s passed (scoped Linux validation; no package/runtime qualification)\n' "$phase"
  fi
  exit "$phase_status"
}
trap verify_after_phase EXIT
cd /work/source
cp /opt/packages.tsv /evidence/packages.tsv
{ uname -a; rustc -Vv; cargo -V; node -v; npm -v; gfortran --version; cmake --version; } > /evidence/toolchain.txt
if [[ $phase == fetch ]]; then
  cargo fetch --locked --manifest-path src-tauri/Cargo.toml --target x86_64-unknown-linux-gnu
  npm --prefix ui ci --ignore-scripts --no-audit --no-fund
elif [[ $phase == contract ]]; then
  # Full upstream gate inventory is recorded, not falsely claimed as executed.
  node scripts/gates --list > /evidence/upstream-gates.txt
  node --test scripts/xnexus/fork-policy.test.mjs
  # A seeded volume deliberately has no old source/node_modules/dist/tsbuildinfo.
  if [[ -f /work/seed.json && ! -d ui/node_modules ]]; then
    npm --prefix ui ci --offline --ignore-scripts --no-audit --no-fund
  fi
  npm --prefix ui rebuild --offline
  npm --prefix ui run build
else
  if [[ -f /work/seed.json ]]; then
    cargo check --locked --offline --manifest-path src-tauri/Cargo.toml --features radio --all-targets --message-format=json-render-diagnostics | tee /evidence/desktop-cargo.jsonl
    python3 /tools/seed_cache.py --check-compiled /evidence/desktop-cargo.jsonl
  else
    cargo check --locked --offline --manifest-path src-tauri/Cargo.toml --features radio --all-targets
  fi
fi

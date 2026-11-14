#!/usr/bin/env bash
# Local unsigned candidate only. No install, launch, DMG, signing or upload.
set -euo pipefail

die() { printf '%s\n' "$*" >&2; exit 1; }
sha256() { shasum -a 256 "$1" | awk '{print $1}'; }

assert_source() {
  local repo=$1 expected=$2 version=$3 origin
  [[ "$expected" =~ ^[0-9a-f]{40}$ ]] || die 'Expected commit must be the full reviewed SHA.'
  [[ "$(git -C "$repo" branch --show-current)" = main ]] || die 'Build only from main.'
  origin=$(git -C "$repo" remote get-url origin)
  case "$origin" in
    https://github.com/oliverbross/OM0RX-xNexus.git|git@github.com:oliverbross/OM0RX-xNexus.git) ;;
    *) die 'Origin must be the authorized OM0RX-xNexus fork.' ;;
  esac
  [[ "$(git -C "$repo" rev-parse HEAD)" = "$expected" ]] || die 'HEAD is not the expected reviewed source.'
  [[ "$(git -C "$repo" rev-parse origin/main)" = "$expected" ]] || die 'Reviewed source must match origin/main.'
  [[ -z "$(git -C "$repo" status --porcelain --untracked-files=all)" ]] || die 'Source must be clean, including untracked files.'
  node - "$repo" "$version" <<'NODE'
const fs = require('node:fs');
const [root, expected] = process.argv.slice(2);
const config = JSON.parse(fs.readFileSync(`${root}/src-tauri/tauri.conf.json`));
if (!/^\d+\.\d+\.\d+-xlog\.\d+$/.test(expected) || config.version !== expected)
  throw Error('Expected version must equal the reviewed fork candidate version.');
NODE
}

# One version can never be reused for another source, even after a failed build.
claim_version() {
  node - "$1" "$2" "$3" <<'NODE'
const fs = require('node:fs'), path = require('node:path');
const [file, source, version] = process.argv.slice(2);
if (!/^[0-9a-f]{40}$/.test(source)) throw Error('Invalid source identity');
fs.mkdirSync(path.dirname(file), {recursive:true});
try { fs.writeFileSync(file, JSON.stringify({version,source},null,2)+'\n', {flag:'wx'}); }
catch (e) {
  if (e.code !== 'EEXIST') throw e;
  const prior = JSON.parse(fs.readFileSync(file));
  if (prior.version !== version || prior.source !== source)
    throw Error('Candidate version already belongs to another source; advance its version.');
}
NODE
}

update_receipt() {
  node - "$1" "$2" "$3" "${4:-}" <<'NODE'
const fs = require('node:fs'), crypto = require('node:crypto');
const [file,status,phase,artifact] = process.argv.slice(2);
const receipt = JSON.parse(fs.readFileSync(file));
receipt.status=status; receipt.phase=phase; receipt.updatedUtc=new Date().toISOString();
if (artifact) receipt.artifact={path:artifact,bytes:fs.statSync(artifact).size,
  sha256:crypto.createHash('sha256').update(fs.readFileSync(artifact)).digest('hex')};
const temp=`${file}.${process.pid}.tmp`;
fs.writeFileSync(temp,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
fs.renameSync(temp,file);
NODE
}

main() {
  [[ $# = 2 || $# = 3 && "${3:-}" = --preflight-only ]] || die 'Usage: bash scripts/xnexus/build-macos.sh <reviewed-40-char-SHA> <candidate-version> [--preflight-only]'
  local expected=$1 version=$2 preflight=${3:-} repo arch target cli_sha os tool floor
  repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
  [[ "$(uname -s)" = Darwin ]] || die 'This script builds on macOS only.'
  os=$(/usr/bin/sw_vers -productVersion)
  [[ "$os" =~ ^([0-9]+)\. ]] && (( BASH_REMATCH[1] >= 14 )) || die 'macOS 14 or newer is required.'
  for tool in git node npm rustup brew cmake ninja pkg-config gfortran python3 curl unzip shasum tar lipo otool codesign; do
    command -v "$tool" >/dev/null || die "Missing build tool: $tool"
  done
  # Refuse dirty or unexpected repositories before contacting a remote.
  assert_source "$repo" "$expected" "$version"
  GIT_TERMINAL_PROMPT=0 git -C "$repo" fetch origin main
  assert_source "$repo" "$expected" "$version"
  [[ "$(rustup run 1.93.1 rustc --version)" = rustc\ 1.93.1\ * ]] || die 'Pinned Rust 1.93.1 is required.'
  local rust_bin
  rust_bin=$(dirname "$(rustup which --toolchain 1.93.1 rustc)")
  export PATH="$rust_bin:$PATH" RUSTC="$rust_bin/rustc" RUSTDOC="$rust_bin/rustdoc" CARGO="$rust_bin/cargo"
  [[ "$(command -v cargo)" = "$CARGO" && "$(command -v rustc)" = "$RUSTC" ]] || die 'Tauri must inherit the pinned Cargo and rustc binaries.'
  # Do not inherit signing credentials, a model bypass, or unrecorded compiler overrides.
  for tool in NEXUS_ALLOW_MISSING_AICW APPLE_SIGNING_IDENTITY APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_API_ISSUER APPLE_API_KEY APPLE_API_KEY_PATH APPLE_ID APPLE_PASSWORD TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER; do
    [[ -z "${!tool+x}" ]] || die "Unset $tool before building this unsigned candidate."
  done
  arch=$(uname -m)
  case "$arch" in
    arm64) target=aarch64-apple-darwin; cli_sha=7734f1d942dbe6e5fea91c1575452f4bf2cc942e6902f2d9d78513baa8527b24 ;;
    x86_64) target=x86_64-apple-darwin; cli_sha=535b790c9d1995f67fdb21296a2abbf8026c7846b206cff9b93984eb740eab8b ;;
    *) die "Unsupported native architecture: $arch" ;;
  esac
  [[ "$(rustup run 1.93.1 rustc -vV | sed -n 's/^host: //p')" = "$target" ]] || die 'Rust host and build architecture differ.'
  floor=$(node -e 'console.log(require(process.argv[1]).bundle.macOS.minimumSystemVersion)' "$repo/src-tauri/tauri.conf.json")
  [[ "$floor" = 14.0 ]] || die 'Review this script when changing the macOS deployment floor.'
  export MACOSX_DEPLOYMENT_TARGET="$floor" CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2 NEXUS_BUILD_JOBS=2
  export FC="$(command -v gfortran)" BOOST_ROOT="$(brew --prefix boost)"
  export PKG_CONFIG_PATH="$(brew --prefix fftw)/lib/pkgconfig" CMAKE_GENERATOR=Ninja
  # Xcode 27's loader rejects stripped 1.93.1 proc-macro dylibs. Keep debug
  # information explicitly, including build dependencies; this increases size.
  export CARGO_PROFILE_RELEASE_DEBUG=2 CARGO_PROFILE_RELEASE_STRIP=none
  export CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG=2 CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_STRIP=none
  local model="$repo/src-tauri/resources/deepcw" hamlib="$repo/src-tauri/resources/hamlib" fftw fortran file
  local model_sha=a7b3b4aa2610d3e88b8c7d4420eceee31be2d0ae595b3413264334f3ffb1eab5
  local metadata_sha=b4342157b90229ee7380e165f3d8036179b80d1e6ae02f2557388b7fcd558c01
  [[ -f "$model/model.onnx" && "$(sha256 "$model/model.onnx")" = "$model_sha" ]] || die 'Stage the pinned DeepCW model; missing or mismatched asset.'
  [[ -f "$model/model.onnx.json" && "$(sha256 "$model/model.onnx.json")" = "$metadata_sha" ]] || die 'Stage the pinned DeepCW metadata; missing or mismatched asset.'
  for file in rigctld rigctl rotctld rotctl COPYING.txt COPYING.LIB.txt LICENSE.txt AUTHORS.txt libusb-COPYING.txt; do
    [[ -s "$hamlib/$file" ]] || die "Missing staged Hamlib asset: $file. Run the pinned preparer with NEXUS_BUILD_JOBS=2."
  done
  fftw="$(pkg-config --variable=libdir fftw3f)/libfftw3f.a"
  fortran=$("$FC" -print-file-name=libgfortran.a)
  [[ -f "$fftw" && -f "$fortran" ]] || die 'Static FFTW and Fortran runtime archives are required.'
  for file in "$fftw" "$fortran"; do lipo -verify_arch "$arch" "$file"; done
  node "$repo/scripts/check-macho-bundle.mjs" --floor "$floor" --arch "$arch" --require rigctld --require rigctl --require rotctld --require rotctl "$hamlib"
  node --test "$repo/scripts/xnexus/fork-policy.test.mjs"
  if [[ "$preflight" = --preflight-only ]]; then
    printf 'Preflight passed: %s %s (%s). No build or package produced.\n' "$expected" "$version" "$arch"
    return
  fi
  local receipts="$repo/target/xnexus-provenance/macos" run receipt='' artifact phase=initialization
  local build_lock="$repo/target/xnexus-macos/build.lock"
  mkdir -p "$(dirname "$build_lock")"
  mkdir "$build_lock" || die "Another or interrupted candidate build owns $build_lock; inspect its evidence before retrying."
  trap 'code=$?; if [[ $code -ne 0 && -n "$receipt" && -f "$receipt" ]]; then update_receipt "$receipt" failed "$phase" || true; fi; rmdir "$build_lock" || true' EXIT
  claim_version "$receipts/$version.json" "$expected" "$version"
  run="$receipts/$version-$(date -u +%Y%m%dT%H%M%SZ)-$$"
  mkdir "$run"
  receipt="$run/provenance.json"
  export CARGO_TARGET_DIR="$repo/target/xnexus-macos/$expected"
  node - "$receipt" "$repo" "$expected" "$version" "$target" "$arch" "$os" "$floor" "$FC" "$fftw" "$fortran" "$cli_sha" <<'NODE'
const fs=require('node:fs'), crypto=require('node:crypto'), cp=require('node:child_process');
const [file,root,source,version,target,arch,os,floor,fc,fftw,fortran,cliSha]=process.argv.slice(2);
const run=(bin,args)=>cp.execFileSync(bin,args,{encoding:'utf8'}).trim();
const digest=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const assets={};
for (const dir of ['src-tauri/resources/hamlib','src-tauri/resources/deepcw'])
  for (const name of fs.readdirSync(`${root}/${dir}`)) {
    const p=`${root}/${dir}/${name}`; if(fs.statSync(p).isFile()) assets[`${dir}/${name}`]=digest(p);
  }
const tools={rustc:run(process.env.RUSTC,['-vV']),cargo:run(process.env.CARGO,['--version']),
  inheritedRustc:process.env.RUSTC,inheritedCargo:process.env.CARGO,
  node:process.version,npm:run('npm',['--version']),xcode:run('xcodebuild',['-version']),
  clang:run('clang',['--version']),gfortran:run(fc,['--version']),cmake:run('cmake',['--version']),
  ninja:run('ninja',['--version']),pkgConfig:run('pkg-config',['--version']),
  homebrew:run('brew',['list','--versions','gcc','fftw','boost','cmake','ninja','pkgconf'])};
fs.writeFileSync(file,JSON.stringify({version,source,origin:run('git',['-C',root,'remote','get-url','origin']),
  startedUtc:new Date().toISOString(),status:'started',profile:'release-with-debug-info',target,arch,hostMacOS:os,
  deploymentTarget:floor,deploymentFloorQualified:false,features:['radio','custom-protocol'],jobs:2,
  developerIdSigned:false,notarized:false,installed:false,uploaded:false,
  signingStatus:'unsigned or ad-hoc only; no Developer ID and no notarization',tools,
  tauriCli:{version:'2.11.5',archiveSha256:cliSha},assets,
  hamlibSource:{version:'4.7.1',sha256:'d197a08a3d5d936d7571ae573f745bbba619e88998742c8267e3fcb0fb3d5974',
    libusbVersion:'1.0.30',libusbSha256:'fea36f34f9156400209595e300840767ab1a385ede1dc7ee893015aea9c6dbaf',
    preparer:'scripts/fetch-hamlib-unix.sh',preparerSha256:digest(`${root}/scripts/fetch-hamlib-unix.sh`)},
  staticLibraries:[fftw,fortran].map(p=>({path:p,sha256:digest(p),architecture:run('lipo',['-info',p])})),
  lockfiles:['src-tauri/Cargo.lock','ui/package-lock.json'].map(p=>({path:p,sha256:digest(`${root}/${p}`)})),
  qualification:'Local host candidate. Homebrew dependencies and static imports do not prove every macOS 14 system.'},null,2)+'\n',{flag:'wx'});
NODE
  # Any failure leaves the source claim and complete logs; a failed artifact is
  # never labelled as packaged. No cleanup or evidence overwrite on retry.
  phase=tools
  mkdir "$run/tools"
  curl --proto '=https' --tlsv1.2 -fL --retry 3 "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v2.11.5/cargo-tauri-$target.zip" -o "$run/cargo-tauri.zip"
  [[ "$(sha256 "$run/cargo-tauri.zip")" = "$cli_sha" ]] || die 'Pinned Tauri CLI archive hash mismatch.'
  unzip -q "$run/cargo-tauri.zip" cargo-tauri -d "$run/tools"
  export PATH="$run/tools:$PATH"
  [[ "$(rustup run 1.93.1 cargo tauri --version)" = 'tauri-cli 2.11.5' ]] || die 'Unexpected Tauri CLI version.'
  phase=dependencies
  npm --prefix "$repo/ui" ci > "$run/npm-ci.log" 2>&1
  phase=build
  (cd "$repo/src-tauri" && rustup run 1.93.1 cargo tauri build --target "$target" --features radio,custom-protocol --bundles app --no-sign -- --locked) > "$run/build.log" 2>&1
  assert_source "$repo" "$expected" "$version"
  local app="$CARGO_TARGET_DIR/$target/release/bundle/macos/OM0RX-xNexus.app"
  [[ -x "$app/Contents/MacOS/OM0RX-xNexus" ]] || die 'Expected app executable was not produced.'
  phase=artifact-audit
  [[ "$(sha256 "$app/Contents/Resources/resources/deepcw/model.onnx")" = "$model_sha" ]] || die 'Packaged model hash mismatch.'
  [[ "$(sha256 "$app/Contents/Resources/resources/deepcw/model.onnx.json")" = "$metadata_sha" ]] || die 'Packaged metadata hash mismatch.'
  for file in COPYING NOTICE resources/hamlib/COPYING.txt resources/hamlib/COPYING.LIB.txt resources/hamlib/LICENSE.txt resources/hamlib/AUTHORS.txt resources/hamlib/libusb-COPYING.txt; do
    [[ -s "$app/Contents/Resources/$file" ]] || die "Missing packaged license: $file"
  done
  node "$repo/scripts/check-macho-bundle.mjs" --floor "$floor" --arch "$arch" --plist "$app/Contents/Info.plist" --system-imports "$repo/scripts/macos-system-imports.txt" --require Contents/MacOS/OM0RX-xNexus --require Contents/Resources/resources/hamlib/rigctld --require Contents/Resources/resources/hamlib/rigctl --require Contents/Resources/resources/hamlib/rotctld --require Contents/Resources/resources/hamlib/rotctl "$app" > "$run/macho-audit.log" 2>&1
  codesign -dvv "$app" > "$run/code-signature.log" 2>&1 || true
  if grep -q '^Authority=' "$run/code-signature.log"; then die 'Unexpected identity-signed app; this route is unsigned only.'; fi
  phase=archive
  artifact="$run/OM0RX-xNexus_${version}_${arch}_UNSIGNED.app.tar.gz"
  COPYFILE_DISABLE=1 tar -czf "$artifact" -C "$(dirname "$app")" "$(basename "$app")"
  tar -tzf "$artifact" > "$run/archive-contents.txt"
  assert_source "$repo" "$expected" "$version"
  update_receipt "$receipt" packaged-unsigned-not-notarized complete "$artifact"
  rmdir "$build_lock"
  trap - EXIT
  printf 'Unsigned local candidate: %s\nBuild receipt: %s\n' "$artifact" "$receipt"
}

# Tests source the guard/provenance functions without starting a build.
if [[ "${BASH_SOURCE[0]}" = "$0" ]]; then main "$@"; fi

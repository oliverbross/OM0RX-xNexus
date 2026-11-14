#!/usr/bin/env bash
# Guard/provenance tests only: no dependency build, app, archive or signing.
set -euo pipefail
script=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/build-macos.sh
source "$script"
bash -n "$script"
fixture=$(mktemp -d "${TMPDIR:-/tmp}/xnexus-macos-script.XXXXXX")
printf 'Preserved script-test evidence: %s\n' "$fixture"
node - "$script" "$fixture" <<'NODE'
const fs=require('node:fs'),cp=require('node:child_process');
const [script,fixture]=process.argv.slice(2);
let count=0;
for(const match of fs.readFileSync(script,'utf8').matchAll(/<<'NODE'\n([\s\S]*?)\nNODE/g)) {
  const file=`${fixture}/embedded-${count++}.cjs`;fs.writeFileSync(file,match[1]);
  cp.execFileSync(process.execPath,['--check',file]);
}
if(count<5)throw Error('Expected source/provenance and artifact-audit scripts were not checked.');
NODE
mkdir -p "$fixture/source/src-tauri"
git -C "$fixture/source" init -q -b main
git -C "$fixture/source" config user.name OM0RX
git -C "$fixture/source" config user.email oliverbross@users.noreply.github.com
printf '{"version":"1.24.1-xlog.1"}\n' > "$fixture/source/src-tauri/tauri.conf.json"
git -C "$fixture/source" add src-tauri/tauri.conf.json
git -C "$fixture/source" commit -qm 'Synthetic build guard fixture'
git -C "$fixture/source" remote add origin https://github.com/oliverbross/OM0RX-xNexus.git
revision=$(git -C "$fixture/source" rev-parse HEAD)
git -C "$fixture/source" update-ref refs/remotes/origin/main "$revision"
assert_source "$fixture/source" "$revision" 1.24.1-xlog.1
reject() { if ( "$@" ) > "$fixture/rejected.log" 2>&1; then die "Guard unexpectedly accepted: $*"; fi; }
reject assert_source "$fixture/source" 0000000000000000000000000000000000000000 1.24.1-xlog.1
reject assert_source "$fixture/source" "$revision" 1.24.1-xlog.2
git -C "$fixture/source" switch -qc codex/wrong-branch
reject assert_source "$fixture/source" "$revision" 1.24.1-xlog.1
git -C "$fixture/source" switch -q main
printf 'operator draft\n' > "$fixture/source/untracked.txt"
reject assert_source "$fixture/source" "$revision" 1.24.1-xlog.1
mv "$fixture/source/untracked.txt" "$fixture/preserved-draft.txt"
git -C "$fixture/source" remote set-url origin https://github.com/kd9taw/Nexus.git
reject assert_source "$fixture/source" "$revision" 1.24.1-xlog.1
claim_version "$fixture/version.json" "$revision" 1.24.1-xlog.1
claim_version "$fixture/version.json" "$revision" 1.24.1-xlog.1
reject claim_version "$fixture/version.json" 0000000000000000000000000000000000000000 1.24.1-xlog.1
printf '{"source":"%s","developerIdSigned":false,"notarized":false}\n' "$revision" > "$fixture/receipt.json"
printf 'synthetic artifact for digest verification only\n' > "$fixture/artifact.txt"
update_receipt "$fixture/receipt.json" packaged-unsigned-not-notarized complete "$fixture/artifact.txt"
node - "$fixture/receipt.json" "$fixture/artifact.txt" <<'NODE'
const fs=require('node:fs'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const [receipt,artifact]=process.argv.slice(2), value=JSON.parse(fs.readFileSync(receipt));
assert.equal(value.status,'packaged-unsigned-not-notarized');
assert.equal(value.developerIdSigned,false); assert.equal(value.notarized,false);
assert.equal(value.artifact.bytes,fs.statSync(artifact).size);
assert.equal(value.artifact.sha256,crypto.createHash('sha256').update(fs.readFileSync(artifact)).digest('hex'));
NODE
update_receipt "$fixture/receipt.json" failed artifact-audit
node -e "if(require(process.argv[1]).status!=='failed')process.exit(1)" "$fixture/receipt.json"
printf 'PASS: source, origin, branch, dirty-tree, version reuse and receipt/hash guards. No candidate built.\n'

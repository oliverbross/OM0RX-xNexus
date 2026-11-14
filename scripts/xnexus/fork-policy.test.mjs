import { readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import test from 'node:test';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const read = p => readFileSync(resolve(root, p), 'utf8').replaceAll('\r\n', '\n');
const config = JSON.parse(read('src-tauri/tauri.conf.json'));
function identity(c) {
  assert.equal(c.productName, 'OM0RX-xNexus');
  assert.equal(c.mainBinaryName, 'OM0RX-xNexus');
  assert.equal(c.identifier, 'sk.om0rx.xnexus');
  assert.equal(c.bundle.createUpdaterArtifacts, false);
  assert.equal(c.plugins.updater, undefined);
}
test('fork identity and update configuration are isolated', () => identity(config));
test('identity guard detects upstream updater and identity regressions', () => {
  assert.throws(() => identity({...config, identifier:'com.kd9taw.tempo'}));
  assert.throws(() => identity({...config, plugins:{updater:{endpoints:['https://example.com']}}}));
});
test('versions agree across desktop and UI manifests', () => {
  const version = config.version;
  assert.match(version, /^\d+\.\d+\.\d+-xlog\.\d+$/);
  assert.equal(JSON.parse(read('ui/package.json')).version, version);
  const lock = JSON.parse(read('ui/package-lock.json'));
  assert.equal(lock.version, version);
  assert.equal(lock.packages[''].version, version);
  assert.match(read('src-tauri/Cargo.toml'), new RegExp(`version = "${version.replaceAll('.', '\\.')}"`));
});
test('no inherited updater plugin can execute in either launch path', () => {
  for (const f of ['src-tauri/src/lib.rs','src-tauri/src/client_mode.rs','src-tauri/Cargo.toml'])
    assert.doesNotMatch(read(f), /tauri_plugin_updater::|^tauri-plugin-updater\s*=/m);
  const s = read('src-tauri/src/lib.rs');
  assert.match(s, /fn self_updates\([^)]*\)[^{]*\{\s*false\s*\}/);
  assert.doesNotMatch(s, /const (GITHUB_RELEASES_URL|VERSION_JSON_URL|BEST_RELEASE_URL)/);
  assert.match(s, /XNEXUS_DATA_DIR/);
  assert.doesNotMatch(s, /var_os\("NEXUS_DATA_DIR"\)/);
  assert.equal(JSON.parse(read('src-tauri/capabilities/default.json')).permissions.includes('updater:default'), false);
});
function guardedJobs(s) {
  const jobs = s.slice(s.indexOf('\njobs:\n') + 7).split(/(?=^  [\w-]+:\s*$)/m).filter(b => /^  [\w-]+:/.test(b));
  assert.ok(jobs.length);
  for (const job of jobs) assert.match(job, /^    if: (?:\$\{\{ )?github\.repository == 'kd9taw\/Nexus'/m);
}
test('every inherited workflow job is restricted to upstream repository identity', () => {
  for (const f of readdirSync(resolve(root,'.github/workflows')).filter(f => f.endsWith('.yml') && !f.startsWith('xnexus-')))
    guardedJobs(read(`.github/workflows/${f}`));
});
test('workflow guard detects a newly unguarded job', () => {
  assert.throws(() => guardedJobs('\njobs:\n  publish:\n    runs-on: ubuntu-latest\n'));
});

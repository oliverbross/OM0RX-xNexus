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
function macPrivateStore(c, companion, store, construction = read('src-tauri/src/xlog_window.rs')) {
  const minimum = c.bundle.macOS.minimumSystemVersion;
  assert.match(minimum, /^\d+(?:\.\d+)*$/);
  assert.ok(Number(minimum.split('.')[0]) >= 14, 'private WKWebView stores require macOS 14');
  assert.match(companion, /#\[path = "xlog_profile_store\.rs"\]\s*mod profile_store;/);
  assert.match(companion, /profile_store::for_current_system\(&root\)\.unwrap_or_else\(\|error\| startup_failure\(&error\)\)/);
  assert.match(companion, /#\[cfg\(target_os = "macos"\)\]\s*let store = Some\(store\);/);
  assert.match(companion, /context\.config_mut\(\)\.app\.windows\.clear\(\);/);
  assert.match(companion, /WebviewWindowBuilder::from_config\(app, &window\)\?;/);
  assert.match(companion, /private_window::build\(builder, root\.join\("webview"\), store\)\?;/);
  assert.match(companion, /WebviewWindowBuilder::data_directory\(self, directory\)/);
  assert.match(companion, /WebviewWindowBuilder::data_store_identifier\(self, identifier\)/);
  assert.ok(companion.indexOf('Outbox::open(root.join("xlog-outbox"))') < companion.indexOf('.setup(move |app|'));
  assert.ok(companion.indexOf('context.config_mut().app.windows.clear();') < companion.indexOf('.setup(move |app|'));
  assert.match(construction, /let builder = builder\.data_directory\(directory\);/);
  assert.match(construction, /Some\(identifier\) => builder\.data_store_identifier\(identifier\)/);
  assert.doesNotMatch(companion, /window\.data_store_identifier =|window\.data_directory =/);
  assert.match(store, /require_supported_version\(version\.majorVersion\)\?;/);
  assert.match(store, /if major < 14\s*\{\s*return Err\(/);
}
test('macOS companion uses a guarded private persistent WebView store', () => {
  macPrivateStore(config, read('src-tauri/src/xlog_mode.rs'), read('src-tauri/src/xlog_profile_store.rs'));
});
test('private store guard detects OS downgrade and shared-store fallback', () => {
  const companion = read('src-tauri/src/xlog_mode.rs');
  const store = read('src-tauri/src/xlog_profile_store.rs');
  const older = structuredClone(config);
  older.bundle.macOS.minimumSystemVersion = '13.0';
  assert.throws(() => macPrivateStore(older, companion, store));
  assert.throws(() => macPrivateStore(config, companion.replace('let store = Some(store);', 'let store = None;'), store));
  assert.throws(() => macPrivateStore(config, companion.replace('context.config_mut().app.windows.clear();', ''), store));
  assert.throws(() => macPrivateStore(config, companion.replace('private_window::build(builder, root.join("webview"), store)?;', 'builder.build()?;'), store));
  assert.throws(() => macPrivateStore(config, companion.replace('WebviewWindowBuilder::data_directory(self, directory)', 'self'), store));
  assert.throws(() => macPrivateStore(config, companion.replace('WebviewWindowBuilder::data_store_identifier(self, identifier)', 'self'), store));
  const construction = read('src-tauri/src/xlog_window.rs');
  assert.throws(() => macPrivateStore(config, companion, store, construction.replace('builder.data_directory(directory)', 'builder')));
  assert.throws(() => macPrivateStore(config, companion, store, construction.replace('builder.data_store_identifier(identifier)', 'builder')));
  assert.throws(() => macPrivateStore(config, companion.replace('profile_store::for_current_system(&root)', 'profile_store::identifier(&root)'), store));
  assert.throws(() => macPrivateStore(config, companion, store.replace('require_supported_version(version.majorVersion)?;', '')));
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
  assert.doesNotMatch(s, /updater_leftovers::spawn|join\("Nexus"\)/);
  assert.match(s, /join\("OM0RX-xNexus SSTV"\)/);
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

// SPDX-License-Identifier: GPL-3.0-only
// Materialize a reviewed merge so the exact recorded config is passed to Tauri.
import assert from 'node:assert/strict';
import { mappedNotices } from './notice-resources.mjs';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const licenseSource = '../scripts/xnexus/nsis/LICENSE-MIT.tauri';
const licenseTarget = 'resources/installer/LICENSE-MIT.tauri';
const expectedNsis = {
  template: '../scripts/xnexus/nsis/installer.nsi',
  installerHooks: '../scripts/xnexus/nsis/ownership.nsh',
};
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
export function mergeDistributionConfig(base, overlay) {
  assert.equal(base.identifier, 'sk.om0rx.xnexus', 'Only the fork may use this installer policy');
  assert.deepEqual(Object.keys(overlay), ['bundle']);
  assert.deepEqual(Object.keys(overlay.bundle).sort(), ['resources', 'windows']);
  assert.deepEqual(Object.keys(overlay.bundle.windows), ['nsis']);
  assert.deepEqual(overlay.bundle.windows.nsis, expectedNsis);
  assert.deepEqual(overlay.bundle.resources, { [licenseSource]: licenseTarget });
  assert.ok(object(base.bundle.resources), 'Require the reviewed source-to-destination resource map');
  assert.ok(Object.values(base.bundle.resources).every(value => typeof value === 'string'));
  assert.ok(!(licenseSource in base.bundle.resources) || base.bundle.resources[licenseSource] === licenseTarget);
  const merged = structuredClone(base);
  merged.bundle.resources = { ...base.bundle.resources, ...overlay.bundle.resources };
  merged.bundle.windows = { ...base.bundle.windows, nsis: { ...base.bundle.windows?.nsis, ...expectedNsis } };
  for (const [source, target] of Object.entries(base.bundle.resources)) assert.equal(merged.bundle.resources[source], target);
  // All non-distribution application/build settings remain byte-for-byte values.
  const restored = structuredClone(merged);
  restored.bundle.resources = base.bundle.resources;
  if ('windows' in base.bundle) restored.bundle.windows = base.bundle.windows;
  else delete restored.bundle.windows;
  assert.deepEqual(restored, base);
  return merged;
}
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
export function prepareDistribution(repo, outputDirectory) {
  for (const name of ['tauri.windows.conf.json', 'tauri.windows.conf.json5', 'Tauri.windows.toml']) {
    assert.ok(!existsSync(join(repo, 'src-tauri', name)), `Review additional platform configuration before distribution: ${name}`);
  }
  const inputs = [
    'src-tauri/tauri.conf.json', 'src-tauri/tauri.distribution.conf.json',
    'scripts/xnexus/nsis/installer.nsi', 'scripts/xnexus/nsis/ownership.nsh',
    'scripts/xnexus/nsis/LICENSE-MIT.tauri', 'scripts/xnexus/nsis/upstream-installer.nsi',
    'scripts/xnexus/build-nsis-template.mjs',
    'scripts/xnexus/notice-resources.mjs',
  ];
  const hashes = Object.fromEntries(inputs.map(path => [path, sha(readFileSync(join(repo, path)))]));
  const base = JSON.parse(readFileSync(join(repo, inputs[0]), 'utf8'));
  const overlay = JSON.parse(readFileSync(join(repo, inputs[1]), 'utf8'));
  const effective = mergeDistributionConfig(base, overlay);
  const bytes = JSON.stringify(effective, null, 2) + '\n';
  mkdirSync(outputDirectory, { recursive: true });
  const configPath = join(outputDirectory, `${base.version}.distribution.config.json`);
  const receiptPath = join(outputDirectory, `${base.version}.distribution.json`);
  writeFileSync(configPath, bytes);
  const receipt = {
    version: base.version, configPath, configSha256: sha(bytes), sourceHashes: hashes,
    baseResources: base.bundle.resources, effectiveResources: effective.bundle.resources,
    baseResourceCount: Object.keys(base.bundle.resources).length,
    effectiveResourceCount: Object.keys(effective.bundle.resources).length,
    noticeResources: mappedNotices(repo, effective),
    policy: 'owned-root-nsis-retain-all-profiles',
  };
  writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + '\n');
  return receiptPath;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  if (!process.argv[2]) throw Error('Pass an isolated provenance output directory; this command never builds.');
  console.log(prepareDistribution(repo, resolve(process.argv[2])));
}

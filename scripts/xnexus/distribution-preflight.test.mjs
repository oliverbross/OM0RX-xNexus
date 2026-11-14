// SPDX-License-Identifier: GPL-3.0-only
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { mergeDistributionConfig } from './distribution-preflight.mjs';
const base = JSON.parse(readFileSync(new URL('../../src-tauri/tauri.conf.json', import.meta.url)));
const overlay = JSON.parse(readFileSync(new URL('../../src-tauri/tauri.distribution.conf.json', import.meta.url)));
test('actual overlay retains every base resource and all app/build settings', () => {
  const result = mergeDistributionConfig(base, overlay);
  assert.deepEqual(result.app, base.app);
  assert.deepEqual(result.build, base.build);
  for (const [key, value] of Object.entries(base.bundle.resources)) assert.equal(result.bundle.resources[key], value);
  assert.equal(Object.keys(result.bundle.resources).length, Object.keys(base.bundle.resources).length + 1);
});
test('resource removal/replacement is rejected', () => {
  const changed = structuredClone(overlay);
  changed.bundle.resources = { '../COPYING': 'discarded' };
  assert.throws(() => mergeDistributionConfig(base, changed));
});
test('hidden application/build overlay is rejected', () => {
  assert.throws(() => mergeDistributionConfig(base, { ...overlay, build: { beforeBuildCommand: 'changed' } }));
});
test('an alternate installer hook is rejected', () => {
  const changed = structuredClone(overlay);
  changed.bundle.windows.nsis.installerHooks = 'elsewhere.nsh';
  assert.throws(() => mergeDistributionConfig(base, changed));
});
test('foreign product identity is rejected', () => {
  assert.throws(() => mergeDistributionConfig({ ...base, identifier: 'com.oliverbross.xlog.integration' }, overlay));
});

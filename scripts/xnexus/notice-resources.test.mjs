// SPDX-License-Identifier: GPL-3.0-only
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, renameSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { mappedNotices, verifyMappedNotices } from './notice-resources.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'xnexus-notice-'));
  t.after(() => rmSync(root, { recursive: true }));
  const repo = join(root, 'repo'), payload = join(root, 'payload');
  mkdirSync(repo); mkdirSync(payload);
  const put = (base, path, bytes) => { const p = join(base, path); mkdirSync(dirname(p), { recursive: true }); writeFileSync(p, bytes); };
  const files = [
    ['licenses/ui/THIRD-PARTY.txt', 'resources/ui/THIRD-PARTY.txt', 'UI terms'],
    ['licenses/rust/THIRD-PARTY.txt', 'resources/rust/THIRD-PARTY.txt', 'Rust terms'],
    ['src-tauri/resources/model/LICENSE.txt', 'resources/model/LICENSE.txt', 'Model terms'],
    ['src-tauri/resources/model/README.md', 'resources/model/README.md', 'Model provenance'],
    ['COPYING', 'COPYING', 'Project terms'],
  ];
  for (const [source, target, bytes] of files) { put(repo, source, bytes); put(payload, target, bytes); }
  const config = { bundle: { resources: {
    '../licenses/ui/*': 'resources/ui/', '../licenses/rust/THIRD-PARTY.txt': 'resources/rust/THIRD-PARTY.txt',
    'resources/model/*': 'resources/model/', '../COPYING': 'COPYING',
  } } };
  return { repo, payload, config, put, retained: mappedNotices(repo, config) };
}
test('exact destinations retain same-basename notices and all mapped provenance text deterministically', t => {
  const f = fixture(t);
  assert.equal(f.retained.length, 5);
  assert.deepEqual(verifyMappedNotices(f.repo, f.config, f.payload, f.retained), f.retained);
  const reversed = { bundle: { resources: Object.fromEntries(Object.entries(f.config.bundle.resources).reverse()) } };
  assert.deepEqual(mappedNotices(f.repo, reversed), f.retained);
});
test('changed bytes at the correct destination cannot be hidden by a correct duplicate elsewhere', t => {
  const f = fixture(t);
  f.put(f.payload, 'resources/ui/THIRD-PARTY.txt', 'NO terms'); // same length as original
  f.put(f.payload, 'elsewhere/THIRD-PARTY.txt', 'UI terms');
  assert.throws(() => verifyMappedNotices(f.repo, f.config, f.payload, f.retained), /changed/);
});
test('missing correct path fails despite a same-basename notice elsewhere', t => {
  const f = fixture(t);
  renameSync(join(f.payload, 'resources/ui/THIRD-PARTY.txt'), join(f.payload, 'THIRD-PARTY.txt'));
  assert.throws(() => verifyMappedNotices(f.repo, f.config, f.payload, f.retained));
});
test('source notice changed since preflight is rejected even if payload matches the new bytes', t => {
  const f = fixture(t);
  f.put(f.repo, 'COPYING', 'changed'); f.put(f.payload, 'COPYING', 'changed');
  assert.throws(() => verifyMappedNotices(f.repo, f.config, f.payload, f.retained), /preflight/);
});
test('Windows case-insensitive destination collision is rejected', t => {
  const f = fixture(t);
  f.config.bundle.resources['../licenses/rust/THIRD-PARTY.txt'] = 'RESOURCES/UI/third-party.TXT';
  assert.throws(() => mappedNotices(f.repo, f.config), /collision/);
});
test('non-notice input cannot overwrite a mapped notice', t => {
  const f = fixture(t); f.put(f.repo, 'src-tauri/resources/data.bin', 'binary');
  f.config.bundle.resources['resources/data.bin'] = 'COPYING';
  assert.throws(() => mappedNotices(f.repo, f.config), /collision/);
});
test('unsafe destinations and unsupported glob forms fail closed', t => {
  const f = fixture(t);
  for (const target of ['../COPYING', 'C:/COPYING', '/COPYING', 'resources/../COPYING', 'COPYING.']) {
    assert.throws(() => mappedNotices(f.repo, { bundle: { resources: { '../COPYING': target } } }));
  }
  assert.throws(() => mappedNotices(f.repo, { bundle: { resources: { '../licenses/**': 'resources/' } } }));
});
test('old receipts cannot silently pass without an inventory', t => {
  const f = fixture(t);
  assert.throws(() => verifyMappedNotices(f.repo, f.config, f.payload, undefined), /lacks notice/);
});

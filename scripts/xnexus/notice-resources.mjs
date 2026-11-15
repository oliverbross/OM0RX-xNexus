// SPDX-License-Identifier: GPL-3.0-only
// Resource coverage helper for the existing distribution preflight/artifact audit.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { lstatSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const slash = path => path.split(sep).join('/');
function checked(root, path, directory = false) {
  const rel = relative(root, path);
  assert.ok(rel && !isAbsolute(rel) && rel !== '..' && !rel.startsWith(`..${sep}`), 'Resource escaped its root');
  let current = root;
  assert.ok(lstatSync(root).isDirectory() && !lstatSync(root).isSymbolicLink(), 'Invalid resource root');
  for (const component of rel.split(sep)) {
    current = join(current, component);
    assert.ok(!lstatSync(current).isSymbolicLink(), `Linked resource: ${current}`);
  }
  assert.ok(directory ? lstatSync(path).isDirectory() : lstatSync(path).isFile(), `Unexpected resource type: ${path}`);
  return path;
}
function destination(value) {
  assert.equal(typeof value, 'string');
  assert.ok(value && !value.includes('\\') && !value.startsWith('/') && !/[:*?\[\]\x00-\x1f]/.test(value), 'Unsafe resource destination');
  const parts = value.replace(/\/$/, '').split('/');
  assert.ok(parts.every(p => p && p !== '.' && p !== '..' && !/[. ]$/.test(p)), 'Aliased resource destination');
  return value;
}
function notice(source) {
  return source.startsWith('licenses/') || /\.(txt|md)$/i.test(source)
    || /^(COPYING|NOTICE|LICENSE|LICENCE|AUTHORS|PATENTS|THIRD-PARTY)([._-]|$)/i.test(basename(source));
}

// Support only the reviewed exact-file and flat directory/* map forms. New
// recursive/directory patterns require explicit review, never silent omission.
export function mappedNotices(repository, config) {
  const root = resolve(repository);
  const map = config?.bundle?.resources;
  assert.ok(map && !Array.isArray(map) && typeof map === 'object', 'Require an explicit resource map');
  const targets = new Set();
  const records = [];
  for (const [pattern, rawTarget] of Object.entries(map).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)) {
    const target = destination(rawTarget);
    assert.ok(!pattern.includes('\\') && !/[?\[\]]/.test(pattern), 'Unreviewed resource pattern');
    const glob = pattern.endsWith('/*');
    const prefix = glob ? pattern.slice(0, -2) : pattern;
    assert.ok(!prefix.includes('*'), 'Unreviewed resource glob');
    const input = resolve(root, 'src-tauri', prefix);
    const files = glob
      ? readdirSync(checked(root, input, true)).sort().map(name => checked(root, join(input, name)))
      : [checked(root, input)];
    assert.ok(files.length, `Empty resource mapping: ${pattern}`);
    if (glob) assert.ok(target.endsWith('/'), 'Flat resource glob needs directory destination');
    for (const file of files) {
      const output = destination(target.endsWith('/') ? target + basename(file) : target);
      const key = output.toLowerCase();
      assert.ok(!targets.has(key), `Windows resource destination collision: ${output}`);
      assert.ok(![...targets].some(other => other.startsWith(key + '/') || key.startsWith(other + '/')), `Resource file/directory collision: ${output}`);
      targets.add(key); // Also check non-notice inputs: they must not overwrite a notice.
      const source = slash(relative(root, file));
      if (notice(source)) {
        const bytes = readFileSync(file);
        records.push({ source, destination: output, bytes: bytes.length, sha256: sha(bytes) });
      }
    }
  }
  assert.ok(records.length, 'No mapped notice resources');
  return records.sort((a, b) => a.destination < b.destination ? -1 : a.destination > b.destination ? 1 : 0);
}

export function verifyMappedNotices(repository, config, payloadRoot, retained) {
  assert.ok(Array.isArray(retained) && retained.length, 'Build receipt lacks notice inventory');
  const actual = mappedNotices(repository, config);
  assert.deepEqual(actual, retained, 'Notice source/map differs from build preflight');
  const root = resolve(payloadRoot);
  for (const entry of actual) {
    const path = checked(root, resolve(root, entry.destination));
    const bytes = readFileSync(path);
    assert.equal(bytes.length, entry.bytes, `Notice payload size mismatch: ${entry.destination}`);
    assert.equal(sha(bytes), entry.sha256, `Notice payload changed: ${entry.destination}`);
    // Exact destination matching, not a same-basename copy in another directory.
  }
  return actual;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 7, 'Expected repository, materialized config, payload, build receipt and output');
  const [, , repository, configPath, payload, receiptPath, output] = process.argv;
  const json = path => JSON.parse(readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
  const result = verifyMappedNotices(repository, json(configPath), payload, json(receiptPath).distribution?.noticeResources);
  writeFileSync(output, JSON.stringify({ schema: 1, verifiedNoticeResources: result }, null, 2) + '\n', { flag: 'wx' });
}

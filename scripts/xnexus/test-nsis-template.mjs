// SPDX-License-Identifier: GPL-3.0-only
// Compile only: this fixture installer is NEVER executed and contains no app.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from 'node:fs';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { createHash, randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const support = process.argv[2];
if (!support) throw Error('Pass the preserved Tauri 2.11.5 generated NSIS support directory.');
const nsis = process.argv[3] || join(process.env.LOCALAPPDATA, 'tauri/NSIS');
const require = createRequire(join(repo, 'target/nsis-template-tools/package.json'));
const Handlebars = require('handlebars');
if (Handlebars.VERSION !== '4.7.8') throw Error('Install the pinned test-only handlebars@4.7.8 under target/nsis-template-tools.');
const root = join(repo, 'target/nsis-template', randomUUID());
mkdirSync(root, { recursive: true });
const hash = p => createHash('sha256').update(readFileSync(p)).digest('hex');
const helpers = {};
for (const name of ['utils.nsh', 'FileAssociation.nsh', 'English.nsh']) {
  copyFileSync(join(support, name), join(root, name));
  helpers[name] = hash(join(root, name));
}
if (helpers['utils.nsh'] !== 'fd766caf19dd635793d14e7a4247c454fbf7a179d50b80947845e0abb30cf35f') throw Error('Unexpected Tauri utility source');
const guard = join(repo, 'scripts/xnexus/nsis/ownership.nsh');
const template = join(repo, 'scripts/xnexus/nsis/installer.nsi');
const payload = join(root, 'OM0RX-xNexus.exe');
writeFileSync(payload, 'INERT TEST PAYLOAD. NOT AN APPLICATION. NEVER EXECUTE.\n');
const output = join(root, 'COMPILE-ONLY-DO-NOT-RUN.exe');
const license = join(repo, 'scripts/xnexus/nsis/LICENSE-MIT.tauri');
const data = {
  compression: 'none', manufacturer: 'OM0RX', product_name: 'OM0RX-xNexus',
  version: '0.0.0-test.0', version_with_build: '0.0.0.0', install_mode: 'currentUser',
  main_binary_name: 'OM0RX-xNexus', main_binary_path: payload, bundle_id: 'sk.om0rx.xnexus',
  copyright: 'Disposable compile fixture', out_file: output, arch: 'x64',
  additional_plugins_path: join(nsis, 'Plugins/x86-unicode/additional'),
  allow_downgrades: 'false', display_language_selector: 'false', install_webview2_mode: 'skip',
  installer_hooks: guard, estimated_size: '1', languages: ['English'],
  language_files: ['English.nsh'], resources: { [license]: ['', 'resources\\installer\\LICENSE-MIT.tauri'] },
  resources_dirs: ['resources', 'resources\\installer'], resources_ancestors: ['resources\\installer', 'resources'],
  binaries: {}, file_associations: [], deep_link_protocols: [],
};
Handlebars.registerHelper('no-escape', value => new Handlebars.SafeString(value));
const rendered = Handlebars.compile(readFileSync(template, 'utf8'), { noEscape: true })(data);
writeFileSync(join(root, 'fixture.nsi'), rendered);
const compiler = join(nsis, 'Bin/makensis.exe');
const result = spawnSync(compiler, ['/V3', join(root, 'fixture.nsi')], { cwd: root, encoding: 'utf8', windowsHide: true });
writeFileSync(join(root, 'compile.log'), (result.stdout || '') + (result.stderr || '') + (result.error?.stack || ''));
if (result.status !== 0) throw Error(`Full template compile failed: ${root}/compile.log`);
writeFileSync(join(root, 'receipt.json'), JSON.stringify({
  status: 'compiled-inert-fixture-never-executed', templateSha256: hash(template),
  guardSha256: hash(guard), helperSha256: helpers, compilerSha256: hash(compiler),
  fixtureSha256: hash(join(root, 'fixture.nsi')), artifactSha256: hash(output),
  source: spawnSync('git', ['-C', repo, 'rev-parse', 'HEAD'], { encoding: 'utf8', windowsHide: true }).stdout.trim(),
  sourceStatus: spawnSync('git', ['-C', repo, 'status', '--porcelain'], { encoding: 'utf8', windowsHide: true }).stdout.trim(),
  rendererVersion: Handlebars.VERSION, rendererLockSha256: hash(join(repo, 'target/nsis-template-tools/package-lock.json')),
  completedUtc: new Date().toISOString(), root,
}, null, 2) + '\n');
console.log(`Full NSIS template compiled without execution: ${root}/receipt.json`);

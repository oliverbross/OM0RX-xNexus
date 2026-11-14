// SPDX-License-Identifier: GPL-3.0-only
// Compile only: this fixture installer is NEVER executed and contains no app.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, existsSync } from 'node:fs';
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
const binary = join(root, 'rigctl.exe');
writeFileSync(binary, 'INERT EXTERNAL BINARY FIXTURE. NEVER EXECUTE.\n');
const data = {
  compression: 'none', manufacturer: 'OM0RX', product_name: 'OM0RX-xNexus',
  version: '0.0.0-test.0', version_with_build: '0.0.0.0', install_mode: 'currentUser',
  main_binary_name: 'OM0RX-xNexus', main_binary_path: payload, bundle_id: 'sk.om0rx.xnexus',
  copyright: 'Disposable compile fixture', out_file: output, arch: 'x64',
  additional_plugins_path: join(nsis, 'Plugins/x86-unicode/additional'),
  allow_downgrades: 'false', display_language_selector: 'false', install_webview2_mode: 'skip',
  installer_hooks: guard, estimated_size: '1', languages: ['English'],
  language_files: ['English.nsh'], resources: { [license]: ['', 'resources\\installer\\LICENSE-MIT.tauri'] },
  resources_dirs: ['resources', 'resources\\installer', 'resources\\hamlib'], resources_ancestors: ['resources\\installer', 'resources\\hamlib', 'resources'],
  binaries: { [binary]: 'resources\\hamlib\\rigctl.exe' }, file_associations: [], deep_link_protocols: [],
};
Handlebars.registerHelper('no-escape', value => new Handlebars.SafeString(value));
const rendered = Handlebars.compile(readFileSync(template, 'utf8'), { noEscape: true })(data);
if (rendered.includes('{{')) throw Error('Unexpanded Handlebars expression in rendered fixture');
writeFileSync(join(root, 'fixture.nsi'), rendered);
const compiler = join(nsis, 'Bin/makensis.exe');
const result = spawnSync(compiler, ['/V3', join(root, 'fixture.nsi')], { cwd: root, encoding: 'utf8', windowsHide: true });
writeFileSync(join(root, 'compile.log'), (result.stdout || '') + (result.stderr || '') + (result.error?.stack || ''));
if (result.status !== 0) throw Error(`Full template compile failed: ${root}/compile.log`);
// Execute only a tiny test harness containing the EXACT rendered marker functions.
// It has no File extraction, registry operations, product entry point or wizard UI.
const functions = ['XNValidateCompletePayload', 'XNInvalidateOwnershipMarker', 'XNWriteOwnershipMarker'].map(name => {
  const start = rendered.indexOf(`Function ${name}\n`);
  const end = rendered.indexOf('FunctionEnd', start);
  if (start < 0 || end < start) throw Error(`Missing marker function ${name}`);
  return rendered.slice(start, end + 'FunctionEnd'.length);
}).join('\n\n');
const probe = join(root, 'marker-probe.exe');
const harness = String.raw`Unicode true
RequestExecutionLevel user
SilentInstall silent
AutoCloseWindow true
Name "Disposable xNexus payload completion test"
OutFile "@PROBE@"
!addplugindir "@PLUGINS@"
!define MAINBINARYNAME "OM0RX-xNexus"
!define XN_TEST_NO_UNINSTALL
!include "@GUARD@"
Function .onInstFailed
  SetErrorLevel 91
FunctionEnd
Section
  @GetOptions@ $CMDLINE "/CASE=" $0
  StrCpy $INSTDIR "@ROOT@\cases\$0"
  Call XNInvalidateOwnershipMarker
  Call XNWriteOwnershipMarker
  SetErrorLevel 0
SectionEnd
@FUNCTIONS@
`.replace('@PROBE@', probe).replace('@PLUGINS@', data.additional_plugins_path)
  .replace('@GUARD@', guard).replace('@ROOT@', root).replace('@GetOptions@', '${GetOptions}').replace('@FUNCTIONS@', functions);
const probeSource = join(root, 'marker-probe.nsi');
writeFileSync(probeSource, harness);
const probeCompile = spawnSync(compiler, ['/V3', probeSource], { cwd: root, encoding: 'utf8', windowsHide: true });
writeFileSync(join(root, 'marker-compile.log'), (probeCompile.stdout || '') + (probeCompile.stderr || '') + (probeCompile.error?.stack || ''));
if (probeCompile.status !== 0) throw Error(`Marker probe compile failed: ${root}/marker-compile.log`);
const payloadPaths = ['OM0RX-xNexus.exe', 'uninstall.exe', 'resources/installer/LICENSE-MIT.tauri', 'resources/hamlib/rigctl.exe'];
const markerCases = [];
for (const [index, missing] of [null, ...payloadPaths, 'upgrade-missing-resource'].entries()) {
  const directory = join(root, 'cases', String(index));
  mkdirSync(directory, { recursive: true });
  for (const path of payloadPaths) {
    if (path === missing || (missing === 'upgrade-missing-resource' && path === payloadPaths[2])) continue;
    const destination = join(directory, path);
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, 'inert fixture; never executable\n');
  }
  const marker = join(directory, '.om0rx-xnexus-owner.ini');
  if (missing === 'upgrade-missing-resource') writeFileSync(marker, `[Ownership]\nProduct=sk.om0rx.xnexus\nFormat=1\nRoot=${directory}\n`);
  const sentinel = join(directory, 'pending-queue-sentinel.txt');
  writeFileSync(sentinel, 'must remain byte-identical\n');
  const before = hash(sentinel);
  const run = spawnSync(probe, [`/CASE=${index}`], { cwd: root, encoding: 'utf8', windowsHide: true, timeout: 30000 });
  const expected = missing === null ? 0 : 91;
  if (run.status !== expected || existsSync(marker) !== (missing === null) || hash(sentinel) !== before)
    throw Error(`Payload marker case ${missing || 'complete'} failed: exit=${run.status}, marker=${existsSync(marker)}, error=${run.error}`);
  markerCases.push({ case: missing || 'complete', exit: run.status, markerPresent: existsSync(marker), sentinelSha256: before });
}
writeFileSync(join(root, 'receipt.json'), JSON.stringify({
  status: 'compiled-inert-fixture-never-executed', templateSha256: hash(template),
  guardSha256: hash(guard), helperSha256: helpers, compilerSha256: hash(compiler),
  fixtureSha256: hash(join(root, 'fixture.nsi')), artifactSha256: hash(output),
  markerCases, markerHarnessSha256: hash(probeSource), markerProbeSha256: hash(probe),
  source: spawnSync('git', ['-C', repo, 'rev-parse', 'HEAD'], { encoding: 'utf8', windowsHide: true }).stdout.trim(),
  sourceStatus: spawnSync('git', ['-C', repo, 'status', '--porcelain'], { encoding: 'utf8', windowsHide: true }).stdout.trim(),
  rendererVersion: Handlebars.VERSION, rendererLockSha256: hash(join(repo, 'target/nsis-template-tools/package-lock.json')),
  completedUtc: new Date().toISOString(), root,
}, null, 2) + '\n');
console.log(`Full NSIS template compiled without execution: ${root}/receipt.json`);

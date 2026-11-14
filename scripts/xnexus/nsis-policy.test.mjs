// SPDX-License-Identifier: GPL-3.0-only
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const template = readFileSync(new URL('./nsis/installer.nsi', import.meta.url), 'utf8');
const guard = readFileSync(new URL('./nsis/ownership.nsh', import.meta.url), 'utf8');
function policy(text) {
  const block = (start, end) => {
    const position = text.indexOf(start);
    assert.ok(position >= 0, start);
    const stop = text.indexOf(end, position + start.length);
    assert.ok(stop > position, end);
    return text.slice(position, stop);
  };
  const install = block('Section Install\n', 'SectionEnd');
  assert.match(install, /^Section Install\n  Call XNValidateInstallerLocations\n  Call XNRequireOrdinaryClosure\n  ClearErrors\n  SetOutPath/);
  assert.match(install, /SetOutPath \$INSTDIR\n  \$\{If\} \$\{Errors\}\n    Abort/);
  assert.ok(install.indexOf('Call XNInvalidateOwnershipMarker') < install.indexOf('File "${MAINBINARYSRCPATH}"'));
  assert.match(install, /AllowSkipFiles off\n  SetOverwrite on\n  ClearErrors\n  File/);
  assert.match(install, /WriteUninstaller[^\n]+\n  \$\{If\} \$\{Errors\}\n    Abort/);
  assert.ok(install.indexOf('Call XNWriteOwnershipMarker') > install.indexOf('WriteUninstaller'));
  const early = block('Section EarlyChecks\n', 'SectionEnd');
  assert.match(early, /Call XNValidateInstallerLocations\n  Call XNRequireOrdinaryClosure/);
  assert.ok(text.indexOf('Section EarlyChecks') < text.indexOf('Section WebView2'));
  const init = block('Function .onInit\n', 'FunctionEnd');
  assert.ok(init.indexOf('Call XNValidateInstallerLocations') < init.indexOf('!insertmacro MUI_LANGDLL_DISPLAY'));
  assert.match(init, /Call XNRequireOrdinaryClosure/);
  const uninstall = block('Section Uninstall\n', 'SectionEnd');
  assert.ok(uninstall.indexOf('Call un.XNValidateRoot') < uninstall.indexOf('\n  Delete '));
  assert.ok(uninstall.indexOf('Call un.XNCheckClosed') < uninstall.indexOf('\n  Delete '));
  assert.match(uninstall, /Delete "\$INSTDIR\\\$\{XN_OWNER_FILE\}"/);
  assert.doesNotMatch(text, /CheckIfAppIsRunning|KillProcess|ReadRegStr[^\n]*UninstallString|ExecWait[^\n]*uninstall\.exe|DeleteAppDataCheckbox|RMDir\s+\/r\b/i);
  assert.match(text, /!define MUI_FINISHPAGE_RUN_NOTCHECKED/);
  assert.match(text, /queued QSOs, and all default, named or custom profiles will be kept/);
  assert.match(text, /MUI_PAGE_CUSTOMFUNCTION_LEAVE XNValidateInstallerLocations/);
  assert.match(text, /ReadRegStr \$XNRetained SHCTX "\$\{MANUPRODUCTKEY\}" ""\n  Call XNValidateLocations/);
  assert.match(text, /\$0 != 0\n    \$\{AndIf\} \$0 != 1/);
  const marker = block('Function XNWriteOwnershipMarker\n', 'FunctionEnd');
  assert.match(marker, /^Function XNWriteOwnershipMarker\n  Call XNValidateCompletePayload\n  \$\{If\} \$XNResult <> 0\n    Abort/);
  const payload = block('Function XNValidateCompletePayload\n', 'FunctionEnd');
  assert.match(payload, /MAINBINARYNAME\}\.exe/);
  assert.match(payload, /uninstall\.exe/);
  assert.match(payload, /\{\{#each resources\}\}/);
  assert.match(payload, /\{\{#each binaries\}\}/);
}
test('complete generated template keeps admission before destructive or external steps', () => policy(template));
test('policy rejects missing install admission', () => assert.throws(() => policy(template.replace('Section Install\n  Call XNValidateInstallerLocations', 'Section Install\n  ; removed'))));
test('policy rejects restored process termination', () => assert.throws(() => policy(template + '\nnsis_tauri_utils::KillProcessCurrentUser "example.exe"\n')));
test('policy rejects restored recursive profile cleanup', () => assert.throws(() => policy(template + '\nRMDir /r "$APPDATA\\profile"\n')));
test('policy rejects restored previous-uninstaller execution', () => assert.throws(() => policy(template + '\nExecWait "uninstall.exe"\n')));
test('policy rejects default autorun', () => assert.throws(() => policy(template.replace('!define MUI_FINISHPAGE_RUN_NOTCHECKED', '; removed'))));
test('policy rejects skippable payload extraction', () => assert.throws(() => policy(template.replace('AllowSkipFiles off', 'AllowSkipFiles on'))));
test('policy rejects marker before complete payload validation', () => assert.throws(() => policy(template.replace('  Call XNValidateCompletePayload', '  ; missing validation'))));
test('both install and uninstall use the same guard implementation', () => {
  assert.match(guard, /!insertmacro XNGuardFunctions ""/);
  assert.match(guard, /!insertmacro XNGuardFunctions "un\."/);
  assert.match(guard, /\$XNSelected != \$XNRetained/);
  assert.doesNotMatch(guard, /KillProcess|WriteReg|DeleteReg|\n\s*Delete |\n\s*RMDir /);
});

# Windows .4 disposable installer acceptance

The real unsigned `1.24.1-xlog.4` installer passed a disposable silent install
and ordinary silent uninstall on 15 November 2026. Application source remained
`ae35fc6435f861e38029049299833afe57de4885`; the [artifact audit](WINDOWS_BUILD_1.24.1-xlog.4.md)
and independently retained files were unchanged. This is installer acceptance,
separate from the subsequent permanent installation and native UI evidence below.
Physical station qualification remains pending.

The exact installer SHA-256 is
`13F3173193FA17EC34685978C1E9218EF69D4A96ED57FE0B1E360C56F46C3A31`.
The receipt is retained under the xLog workspace at
`.build/xnexus-installer-acceptance/acceptance-734e9258f09e485ba16f20db1c708adf/transaction.json`,
SHA-256 `281A97EDB2E5CA879C8256517CA15ECADE06B618CC63A187E7E7C155CA6AFE39`.
Its status is `passed-silent-install-uninstall`, final stage `retention-verified`.

Installation used `/S /NS /D=<fresh acceptance root>\app`; uninstall used `/S`.
Fresh namespace, payload, profile and ownership checks preceded dispatch.
Both operations exited naturally with root exit 0 and zero active descendants.
The observer did not terminate either operation. The receipt records both
`installedVerified` and `uninstalledVerified` as true, and application launch
requested/launched and forced cleanup as false. Final verification completed
at `2026-11-15T01:27:32.2355008Z`.

The verified application payload and product registration were removed.
Unknown root/nested sentinel files, separate synthetic draft/outbox profiles,
existing shortcuts and the WebView runtime were retained. This does not claim
all unrelated shell Recent history was inspected or unchanged. No application
or hardware action was part of this transaction.

Earlier receipts remain preserved: `acceptance-2217357a0e4e45c89b432f6332ef781f`
stopped before transaction; `acceptance-5b4c0947b41c4186bc943918dbaadf04` and
`acceptance-b2683605c53b4b27816273113a2172f9` stopped preserving state. Their
outcomes have not been rewritten as passes. The completed run used separately
reviewed JSON, native process-identity and exact NSIS child-argument corrections.

The final transaction tooling is preserved at
`e789d655e609f82c66ccc1b85cb0451e97fd2ead`. Its ledger merge passed 60 inert
acceptance controls with the actual distribution configuration, 51 inert
transaction controls, all PowerShell parsing, eight fork policy tests and
the three documentation gates. Native observer runtime fixtures had already
passed on that tooling branch; they were not rerun for the documentation merge.
See the [acceptance plan](WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md) and
[maintained preparation tool](WINDOWS_INSTALLER_PREPARATION_TOOL.md).

## Native private profile and lifecycle acceptance

The unchanged standalone executable passed native A→B→A acceptance. Both new
profiles started blank, retained different synthetic drafts, and returning to A
restored its exact call/comment plus request/contact identifiers. All three
ordinary Alt-F4 closes completed; the candidate and its private WebView processes
exited, and the synthetic owner stopped normally. The checks used accessibility
observations; graphics capture timed out, so no screenshot qualification is claimed.
No pairing, QSO submission or hardware action occurred.

Receipt `.build/xnexus-installer-acceptance/native-ui-A-B-A-9e1ffa.json` has
SHA-256 `D5182423C6DB8275DF4D5AFA4A91326D2B8ED2D8D894E0C8F29B1AB2C64C9FB7`.
It binds source `ae35fc6435f861e38029049299833afe57de4885` and standalone hash
`44C2BE04218E643266A44A87185DB4969ABAB711B05EB5FC4E90EAFFE7C88ED8`.
The initial A-return stopped before launch because the observer compared two
representations of the same UTC instant. The reviewed tooling correction
`d48805119003ac887d4efcfeb4a803751b94c1f0` compares UTC ticks and explicitly pins
the prior launcher receipt; all 31 guard controls passed. The stopped attempt
remains preserved and was not an application failure.

## Permanent current-user installation

The exact audited installer subsequently passed installation into
`C:\Users\Oliver\AppData\Local\OM0RX-xNexus` using `/S /D=<target>`.
The receipt records `passed-current-user-install-no-app-launch` and
`owned-installed-verified`: all 37 payload files, ownership marker and PE
uninstaller (39 files total), HKCU registration and two shortcuts were verified.
Profiles, unrelated shortcuts and the WebView runtime were preserved. Installer
PID 25672 exited 0 naturally; total processes 1, active processes 0, no forced
cleanup. The installed application was left closed.

Receipt under the xLog workspace:
`.build/xnexus-user-install-ae35-linkfix/installation-f31169c7540c487d97c3bbc192882fd2/installation.json`,
SHA-256 `B0FDE05A0AC24A2F6F9427F021914515E855F2AFC97EE09935C1A825B1D813DC`.
The reviewed helper hash is
`F062FDCC81728B4CBF93B7A82045D0C977011476C510ADC3DA535CE7096141BC`.
Its correction permits spaces in expected shortcut paths, without changing
application/package bytes. The earlier attempt stopped before dispatch; receipt
`3403E3421DA52B94C4D0D7F0D5AE7E11A89C36DE11B13FA98E4788315ECAD684`
remains preserved. A read-only preflight and five helper guards passed before
the completed attempt.

Native pairing/decoded-needs UI, live DAX reception, physical radio/rotator
operation and installed station lifecycle remain separate pending gates.
Unsigned installation does not establish signing or distribution qualification.

## Subsequent normal installed launch and pairing handoff

On 15 November 2026 the installed application was explicitly launched without
arguments. Process-only verification completed at `2026-11-15T02:07:37.1504623Z`
before accessibility binding: candidate PID 19740, WebView browser PID 20148,
the normal AppData WebView profile, and exact installed executable SHA-256
`B8BBD58EF45DA82B06D9514E43A3DB3B151F0D60506EB45CFD153BFD4486A9BE`.
Accessibility showed version `.4`/source `ae35` and a blank unpaired view.

Receipt `.build/xnexus-installer-acceptance/installed-normal-launch-cd403fcc627d4b9195fbb46fa6f231fb/launch.json`
has SHA-256 `C069F4EB804E572976B8BD256C771CFA51E750403939789D1FF08F7B5A960D1A`.
This verifies normal installed process/profile coordinates, not paired reception.

The operator manually enabled companion pairing in xLog. Fresh owner observations
showed its localhost endpoint/Revoke control and DAX Audio RX 1 at 48,000 Hz/two
channels; current slice A selection/capture had not started at that observation.
One xNexus pairing-button click failed because coordinate geometry was unavailable.
Both apps were left open awaiting operator selection of the native pairing file,
current A/RX1 confirmation/sharing and FT8 Start receiving. No pairing credential
or credential path belongs in this repository. Paired UI/live RX remain unqualified.

# Windows .4 disposable installer acceptance

The real unsigned `1.24.1-xlog.4` installer passed a disposable silent install
and ordinary silent uninstall on 15 November 2026. Application source remained
`ae35fc6435f861e38029049299833afe57de4885`; the [artifact audit](WINDOWS_BUILD_1.24.1-xlog.4.md)
and independently retained files were unchanged. This is installer acceptance,
not permanent installation, native UI or physical station qualification.

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

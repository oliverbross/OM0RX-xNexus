# 1.24.1-xlog.8 testing candidate

Application source: `ffac0c95cc5fe7c8eea0290393c2d030923ac3f2`.
This commit advances version metadata only and records this candidate. The exact
resulting clean version commit is bound by the packaging proposal and emitted
build receipt before compilation; those receipts also identify all installers.

Changes: original Nexus FT8/FT4 transmit controls through xLog, original native
message encoder and sequencer, DAX TX playback through the sole xLog owner,
worked/needed/new-band/new-mode/DXCC badges, and working Settings navigation.
User-confirmed mapping is FLEX slice A, DAX TX; RX remains DAX RX1.

Build: canonical C:/Projects/OM0RX-xNexus; scripts/xnexus/build-windows.ps1,
exact ExpectedCommit, Rust1.93.1 Windows GNU, locked release profile with
radio,custom-protocol, j1, original fat LTO, original2560MiB frontend heap.
The existing bounded11GiB build profile is retained. No additional broad test
round or separate full native check is scheduled; packaging supplies the final
integrated native/frontend compile. Focused source evidence is recorded in
ORIGINAL_FT_CONTROLS.md and OWNER_FT_HOST.md.

This is a testing candidate. No live RF transmission was used for qualification.
xLog must commission the explicit output/slice and allow digital TX; Nexus then
requires its own TX enable. Pair again after the owner restarts. Do a supervised
low-power check of DAX level, PTT release and QSO logging before routine use.
Upstream publishing/deployment workflows remain disabled.

The .7 preflight stopped before compilation because the desktop lock omitted
the SDK sha2 dependency edge. Commit ffac0c95 records that existing locked
0.10.9 dependency, with no package upgrades or new feature. This fresh .8
version retains all previous evidence and reruns only actual packaging.

# 1.24.1-xlog.9 receive-continuity testing candidate

Application source: ef20cf49e9d5c71c6fd02ef3a0fd013a014012ce.
This commit advances version metadata only and records the new candidate.
The exact resulting clean version commit is bound in the package proposal
and build receipt before compilation.

Fixes for the three reported problems:
- Refresh xLog classifications before expiry. Keep still-valid facts through
  transient reads without extending their original leases.
- Keep decoded rows during an audio packet gap on the identical route.
  Retire partial DSP/FT/spectrum work; genuine frequency/mode/capture changes
  and owner revocation still clear data.
- Display the actual TX setup refusal instead of calling FT8 receive-only.
  Failed Work/TX action messages persist through successful status polls.

Focused verification: four SDK plus six actual native Worker regressions;
nineteen UI receiver/transmitter/operating-strip regressions. Red reproductions
and the caught first-edit error are preserved. No broad test campaign, new
interface or new transmit engine was added. The package build provides final
TypeScript/native compilation.

Compatible existing owner: installed xLog1.6.43, source
4fb94000469b56ccdd794300b555f3d875fea509. No xLog rebuild/restart is needed
for these Nexus fixes. The operator must save the confirmed FLEX A/DAX TX
route and allow local TX in xLog before Work/double-click can transmit.
Single-click remains selection, matching the original interface.

Build canonical C:/Projects/OM0RX-xNexus with the existing build-windows.ps1,
locked Rust1.93.1 Windows GNU, release fat-LTO, radio/custom-protocol, j1,
2560MiB frontend heap,11GiB job and unchanged runtime floors.
No RF transmission was used for source qualification.

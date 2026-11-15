# Offline sequence module validation

Tested 15 November 2026 in `codex/xlog-offline-sequence`, based on exact Nexus
`4d21abcf73636a8cbbeecb09fb0c1f603789f81e`. Runtime source SHA256:
`E20BEC8D4ECE045AAAAA80874D9E594CD7963CE75E3948F45E7E2D3E66F9B82A`;
22-test source SHA256:
`1FE99D010BEC4995DEA69D15A58E1A82358D34F03C2DD71C52BC16DED2E87414`.

The exact fresh modules passed **22 sequence tests plus six original preparation
goldens**, including actual Station exchanges in FT8 and FT4, dropped Busy period
fencing, duplicate/conflicting periods, frozen state, exact fake-start correlation,
release retention, budgets, one-shot closing messages, slot overlap and bounds.
Golden tests retain independent vendored waveform/tone comparisons. Synthetic
decode measurements and caller-supplied fake start times are explicitly offline:
this component has no real clock and does not observe physical transmission.

Evidence directory in the isolated worktree:
`target/sequence-attempt-04006dffd0b64e229bbc6b28d76e2570`.

| Evidence | SHA256 |
|---|---|
| Final receipt | `0E11EAF98C21C20A04651556E82090843C87B57F90A74C49870586BF204D564E` |
| Guard | `534F503EC22410EF4F90BFE3427751CCB9F445F0BC13C4049251BA09CFDADB60` |
| Exact checks | `86950BF9ADB51805A97176221A1BE74E99DCCE13A237ED6209DBC1088F5D5362` |
| Sequence test log | `F1B51C4FA18F2C087848DA457BEE3164EB0FE4C738755EC7E578F5E2D18BDCD5` |
| Preparation golden log | `CE5A18DD5DAA33A6960DB06BB18D7F584D8E3DA79668CC8D4523284153ECF967` |
| Reviewed input manifest | `351FB58B1438E3E08A901F1CC215909AE6F70A9326A6EDE94E9F7AF58CCE49D6` |

The unchanged corrected supervisor used an aggregate 2 GiB cap, 300 second
deadline, fresh 8 GiB physical / 6 GiB commit admission, 2 GiB runtime floors,
BelowNormal priority and five-second natural drain. Direct compiler calls ran
sequentially with one codegen unit. Guarded root and descendants exited naturally:
exit 0, final active count 0, 20 ms drain, no forced cleanup, memory or abnormal
exit notification. Peak job memory was 619094016 bytes; minimum available physical
memory 20014120960 bytes and commit 18452275200 bytes.

GNU Rust 1.93.1 freshly compiled the actual preparer library/golden executable and
sequence test executable. The 1746 selectable retained native library files were
read-only and hashed before/after, as were 16 source and 50 tool files. The
explicit modes, FT8, FT4, SHA2 and tempo-core libraries matched prior qualified
receipts. Entire `crates/`, `libtempo/` and root lock content matched dependency
source `43e305f1d65ec27f6f72255a14f2eb0775e9b349`. No Cargo build, CMake run,
dependency rebuild or application launch occurred. This is not a full Cargo,
Clippy, desktop, platform or installed qualification.

## Isolated package graph

The crate has its own workspace and no desktop registration. After the runtime
tests, its lock and full GNU-target metadata were checked offline without a
compiler. Initial standalone resolution selected newer locally cached versions;
that graph was **not adopted or compiled**. Its initial lock (`333D5DD08F9852AA70C1F9A0F228F98C9C37C2996AFF859B62BB46974B721D5A`),
metadata and failed equivalence check remain in
`target/offline-sequence-package-graph`.

The final isolated lock was seeded from the existing shell lock plus the two
actual local adapter packages, then resolved and pruned by offline Cargo. Locked
offline full metadata passed for `x86_64-pc-windows-gnu`; all 96 registry identities
and checksums in the 113-package lock are present in the existing shell lock.
The sole workspace member and both actual path dependencies were verified.
This metadata check does not imply a fresh compilation of that Cargo graph.

| Final graph evidence | SHA256 |
|---|---|
| Isolated Cargo.lock | `23196FEE5090CC399F26BAA17E609CB841F56A80909C2E0B2DD13D91D4E63386` |
| Locked full metadata | `0F4BFF7A7DFEF90D568E752A047FD382C2E1E249F69F509D8DAE2D2C76E15133` |
| Graph checks | `3CEE7C27F9DDF3EAE27CB3F2C8419DBCB1A241976C3E6294E41362E269576B16` |

## Owner and operating gates remain separate

The next fixture gate imports actual owner Body and the independent period
validator, using all fourteen retained raw manifests/PCM cases. It is not yet
run. That gate's planned scope is fake prepare/commit/observation correlation,
not an integrated validated-PCM transfer into a commissioned owner backend.
The owner's pending raw-manifest/body-identity and Permit corrections require
their own pinned source and later integrated replay.

No TX endpoint, waveform output, physical timing, watchdog/release, AP/memo change,
logging, native UI, version bump or package is included. Process-restart
durability belongs to the owner journal; a new adapter is disarmed. M6 remains
unfinished software and requires separate supervised station acceptance.

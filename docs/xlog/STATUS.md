# Execution ledger — 14 November 2026

The integration remains in qualification. Source, synthetic tests, packaged
binaries and physical station acceptance are separate evidence levels.

| Milestone | Current evidence | Remaining acceptance |
|---|---|---|
| M0 | Complete: full upstream history, private fork, fetch-only upstream, nine tracked milestones | Maintain upstream review and patch inventory |
| M1 | 58 client tests; actual owner HTTP and SQLite restart/receipt tests; bridge merged into canonical xLog | Paired installed-application acceptance |
| M2 | Separate companion startup, native pairing and private profile; no local station services | Final candidate native GUI and lifecycle acceptance |
| M3 | xLog-authoritative needed spots, band/world maps, selected-call history | Paired application classification/lifecycle acceptance; decoded-call batch classification |
| M4 | Owner-mediated tune/point and independent rotator Stop; identity/revision/freshness checks | Native UI acceptance and physical control qualification |
| M5 | Durable outbox and atomic owner write/receipt; restart, replay and edit-conflict tests | Paired native GUI acceptance; queue archive/export design |
| M6 | Explicit FLEX slice A/DAX mapping, bounded owner PCM, client, FIR/period assembler, threaded native FT8/FT4 decoder and receive UI | Active DAX reception and clock/route qualification; TX contract and supervised qualification |
| M7 | Distinct app/profile/update identity, pinned assets and exact-source packaging scripts | Windows .2 package, isolated application and installer acceptance |
| M8 | Core Mac native checks passed; core .1 app compiled; bundle audit held | Mac bundle/lifecycle, Linux builds, signing and distribution qualification |

RX runtime integration source is `e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c`.
Windows candidate `1.24.1-xlog.2` adds candidate manifests and these records;
its packaging receipt records the exact final commit. The earlier Windows .1
build at `5e56be94c7ede2099d5b8ed6d1a4d0d1ad12134b` was stopped before the
application build and produced no application or installer. Its logs and aborted
receipt are retained. The Mac .1 build remains frozen at that original source.

See [RX_CLIENT_VERIFICATION.md](RX_CLIENT_VERIFICATION.md),
[RX_DESKTOP.md](RX_DESKTOP.md) and [VERIFICATION.md](VERIFICATION.md).
RX evidence includes 58 SDK tests, 40 distinct companion UI tests, TypeScript
and Vite builds, 14 browser viewport/theme cases, 14 pure DSP/assembler tests,
five native adapter tests including both modes at five rates, two actual owner
HTTP-to-native-decoder tests, and three threaded worker/authority tests.
These sets cover different boundaries and are not a combined coverage claim.

## xLog coordination

Canonical xLog contains the bridge and receive owner at
`5efffc1a23af68157e3c564eb2aa345256bea90a`. Validation includes 1,154 native
tests, 30 selected Swift tests, 33 focused bridge/audio checks, six actual
isolated FLEX metadata checks and three receive UI checks. Detailed receipts
are retained in the xLog audio worktree. No production device was captured.

The installed accepted xLog remains 1.6.23. The “xlog work” chat owns canonical
source and the combined 1.6.25 package/install, incorporating its settings,
performance and UI fixes alongside this integration. The unused 1.6.24
reservation was released. xNexus does not independently package or install xLog.

## CI and upstream

Actions is enabled following the operator's approval of the reviewed validation
workflow. Inherited release/deployment workflows remain individually disabled
and guarded by exact upstream repository identity. No billing setting changed.
[Run 43586230218](https://github.com/oliverbross/OM0RX-xNexus/actions/runs/43586230218)
was blocked before execution by payment/spending restrictions. Local checks
provide recorded evidence, not a hosted CI success.

A real isolated upstream merge rehearsal from pin `c88740a94` to available
`3885df0fe` remains preserved for review. It has not been promoted; the shipped
source pin remains unchanged.

## Operational boundaries

The operator confirmed FLEX slice A and DAX receive audio. The DAX channel comes
from actual FLEX metadata and explicit stable endpoint selection; channel 1 is
not a default. DAX devices are registered locally, but a read-only active-input
probe found ten active inputs and none DAX. No DAX utility or stream was started.

Receive timing reports host endpoint clock calibration, not a measured global
UTC accuracy or endpoint-to-RF delay. Both latter values remain explicitly
unknown. Generated-message tests do not establish physical reception.
Decoded rows do not yet have authoritative batch need classification. The
needed-station board continues to use xLog's classified cluster observations.
Transmission is unavailable in companion mode.

The native outbox retains up to 10,000 operations, including terminal receipts.
An archive/export flow remains a long-term operational item.

Full Windows GNU all-targets compilation passed on exact RX runtime `e3ae1534`
with pinned Rust 1.93.1 and locked dependencies. The isolated receipt verified
original dependency hashes and unchanged runtime source. The Mac core application
compiled and bundled at `5e56be94c`, but its archive remains held at the existing
Hamlib/system-symbol allowlist audit; no portable bundle acceptance is claimed.

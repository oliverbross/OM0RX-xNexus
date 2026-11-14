# Execution ledger — 14 November 2026

The integration remains in qualification. Source, synthetic tests, packaged
binaries and physical station acceptance are separate evidence levels.

| Milestone | Current evidence | Remaining acceptance |
|---|---|---|
| M0 | Complete: full upstream history, private fork, fetch-only upstream, nine tracked milestones | Maintain upstream review and patch inventory |
| M1 | 58 client tests; actual owner HTTP and SQLite restart/receipt tests; bridge merged into canonical xLog | Paired installed-application acceptance |
| M2 | Separate companion startup, native pairing and private profile; no local station services | Final candidate native GUI and lifecycle acceptance |
| M3 | xLog-authoritative call/DXCC needs, band/world maps, selected-call history | Paired classification matrix; decoded-call batch lookup; richer confirmation/award policy evidence |
| M4 | Owner-mediated tune/point and independent Stop; connection-bound Stop fix passes 95 focused tests | Fix native integration/accepted xLog build; paired UI and physical control qualification |
| M5 | Durable outbox and atomic owner write/receipt; restart, replay and edit-conflict tests | Paired native GUI acceptance; queue archive/export design |
| M6 | Explicit FLEX slice A/DAX mapping, bounded owner PCM, client, FIR/period assembler, threaded native FT8/FT4 decoder and receive UI | Active DAX reception and clock/route qualification; TX contract and supervised qualification |
| M7 | Distinct app/profile/update identity, pinned assets and exact-source packaging scripts | Windows .2 package, isolated application and installer acceptance |
| M8 | Core Mac .1 archive and private draft/lifecycle checks; exact .2 Mac all-targets check passed | Matching .2 package/paired UI, Linux desktop, signing and distribution qualification |

RX runtime integration source is `e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c`.
Windows candidate `1.24.1-xlog.2` adds candidate manifests and these records;
its exact source is `c2c917b6f748366fbf6b3960876d9f98ba1854c9`. Windows and
Mac optimized builds remain active at this checkpoint; no completed `.2` artifact
is claimed. The earlier Windows .1
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

The “xlog work” chat accepted and installed xLog 1.6.26 from
`df8d81c9d8b9c760fe7a3eee0f25990cb78db732`. Its combined source passed
1,155 Rust tests, 60 reference/Chaser tests and 34 FLEX tests, plus focused UI
checks. A quiet installed check recorded 1,571 station replies in 365 seconds,
with no transport failures or worker retirements. The failed 1.6.25 Settings
bundle-size gate was resolved by loading changes, without raising the limit.
The unused 1.6.24 reservation was released. xNexus does not independently package
or install xLog.

Further review found that retained rotor Stop identified only the driver family.
The isolated fix at `eacfd0c65255023d15ada3b557cb77edac2408bf` binds it to the
observed connection and actual worker. It also cancels queued automatic turns
before waiting for an admitted driver, without revoking a replacement route.
Root and xLog owner source reviews passed; all 95 focused production-source
tests passed. Full native owner validation is queued behind Windows linking.
This fix is not yet integrated into an accepted installed xLog build.

The private Windows xLog setup passed host/worker identity and accessibility
checks, but capture/input geometry failed and keyboard navigation did not move
focus. That sequence remains incomplete, with evidence retained; production
xLog was untouched. Desktop availability clarification is pending.

## CI and upstream

Actions is enabled following the operator's approval of the reviewed validation
workflow. Inherited release/deployment workflows remain individually disabled
and guarded by exact upstream repository identity. No billing setting changed.
[Run 43586230218](https://github.com/oliverbross/OM0RX-xNexus/actions/runs/43586230218)
was blocked before execution by payment/spending restrictions. Local checks
provide recorded evidence, not a hosted CI success.

A new isolated upstream merge rehearsal from exact `.2` source to
`3885df0fe0d992a52f51ff5481025f860b2322f5` had no conflicts across 29 files.
Eight policy checks passed and all 44 inherited workflow jobs remain guarded.
The uncommitted merge and earlier rehearsal remain preserved; no upgrade was
promoted and the upstream base remains `c88740a94`.

Linux provisioning and the initial 58 SDK/eight policy tests passed. TypeScript
then exhausted Node's default 2 GB heap. A 4 GB retry is prepared inside the same
6 GB isolated container and awaits memory headroom; full desktop checking has
not run. Eleven tooling tests now verify exact committed archive/cache bytes,
before/after source integrity, and failure on interrupted memory protection.

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
original dependency hashes and unchanged runtime source. The Mac core `.1`
archive passed the supplemental seven-file Mach-O audit. Private native checks
verified visible identity, readable windows, four ordinary Quits and recovery
of an unqueued synthetic draft. Theme/scale are session-only; draft persistence
passed independently. Historical `.1` CLI path provenance remains incomplete:
the downloaded official CLI is not proven to have been invoked.

The running Mac `.2` build positively uses a locally cargo-installed Tauri 2.11.5
CLI, whose path/hash and installation metadata are retained, with application
compiler Rust 1.93.1. A supplemental record corrects the original downloaded-CLI
assumption. The future build helper now invokes the verified absolute CLI path.
These are local unsigned candidates, with no Developer ID/notarization claim.
See [the detailed checkpoint](QUALIFICATION_2026-11-14.md) for artifact hashes,
qualification limits and preserved failed attempts.

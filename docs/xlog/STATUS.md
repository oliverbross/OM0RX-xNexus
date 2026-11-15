# Execution ledger — 15 November 2026

The integration remains in qualification. Source, synthetic tests, packaged
binaries and physical station acceptance are separate evidence levels.

[Candidate 1.24.1-xlog.4](CANDIDATE_1.24.1-xlog.4.md) has a completed Windows
build, static artifact audit and independent retention from clean canonical
`ae35fc6435f861e38029049299833afe57de4885`. The [disposable install/uninstall](WINDOWS_INSTALLER_1.24.1-xlog.4.md)
passed; permanent installation and native UI acceptance remain in progress.
It remains unsigned. [The build record](WINDOWS_BUILD_1.24.1-xlog.4.md) binds the
actual tools, reviewed 12 GiB supervisor, artifacts and warning qualification.
Canonical source remains frozen during acceptance. The xLog owner reports
1.6.27 packages, installation and installed qualification passed. Its current
documentation is `7789c569` over application source `16cac867`. xLog is currently
closed; ordinary owner handoff and live pairing remain pending.

**Preserved isolation failure:** the fresh Mac `.2` profile displayed a draft from
the prior `.1` test before pairing. Both processes exited normally and no station
action occurred. The locked Tauri configuration conversion drops the requested
WebView storage fields; explicit window construction now repairs that path.
Do not launch or distribute `.1`/`.2` as isolated-profile candidates. See
[the incident and regression gates](PROFILE_ISOLATION_2026-11-14.md).

The repaired [candidate `.3`](CANDIDATE_1.24.1-xlog.3.md), exact source
`d206b880216543af2142e600ea6b8e5ac6bdd1b2`, passed its Mac artifact audit and
native A→B→A isolation checks: two blank starting profiles retained different
drafts in their actual requested WebKit stores, and reopening A restored its
exact identifiers. Three ordinary Quits completed; default-store bytes remained
unchanged. Core paired synthetic acceptance also passed: native pairing,
needed board, simulated Tune/Point/Stop, history and a durable native queue
across Quit/relaunch, followed by exactly one applied synthetic create receipt.
Both app runs and the synthetic owner exited normally. The actual Mac
owner transport does not advertise `audio.rx`; this verifies absent-capability
handling, not Mac receive qualification. The candidate freeze is released;
new implementation still requires its own source tests and candidate qualification.

| Milestone | Current evidence | Remaining acceptance |
|---|---|---|
| M0 | Complete: full upstream history, private fork, fetch-only upstream, nine tracked milestones | Maintain upstream review and patch inventory |
| M1 | 80 client tests plus exact replay of 40 Swift needs replies; actual owner HTTP and SQLite restart/receipt tests; base bridge merged into canonical xLog | Paired installed-application acceptance |
| M2 | Separate companion startup, native pairing and private profile; no local station services | Final candidate native GUI and lifecycle acceptance |
| M3 | xLog-authoritative call/DXCC needs, band/world maps, selected-call history; owner/SDK/native/UI tests and integrated Windows all-targets check passed; `.4` Windows artifact audited | Paired UI; installed classification propagation; richer confirmation/award policy evidence |
| M4 | Owner-mediated tune/point and independent Stop; connection-bound Stop fix merged after 95 focused and 1,164 full native tests; xLog 1.6.27 installed qualification passed | Paired UI and physical control qualification |
| M5 | Durable outbox and atomic owner write/receipt; restart, replay and edit-conflict tests | Paired native GUI acceptance; queue archive/export design |
| M6 | FLEX slice A → DAX Audio RX 1 selected; bounded owner PCM, client, FIR/period assembler, threaded native FT8/FT4 decoder and receive UI; generated FT4 to needs chain passed | Confirm live mapping/endpoint; active reception and clock/route qualification; TX implementation and supervised qualification |
| M7 | Distinct identity, repaired profile construction, reviewed installer guards; `.4` Windows build, audit, retention and disposable install/uninstall passed | Isolated application and permanent installation acceptance |
| M8 | `.4` Windows artifact audited; Mac .3 A→B→A isolation and exact .3 Linux contract/desktop checks passed | Updated Linux/Mac checks and packages, paired UI, native lifecycle, signing and distribution qualification |

RX runtime integration source is `e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c`.
Windows candidate `1.24.1-xlog.2` adds candidate manifests and these records;
its exact source is `c2c917b6f748366fbf6b3960876d9f98ba1854c9`. Windows and
Mac optimized builds completed. The Windows installer passed its static
audit. The Mac archive passed its supplemental artifact
audit, then failed native profile isolation. Neither `.2` candidate is qualified
for use. The earlier Windows .1
build at `5e56be94c7ede2099d5b8ed6d1a4d0d1ad12134b` was stopped before the
application build and produced no application or installer. Its logs and aborted
receipt are retained. The Mac .1 build remains frozen at that original source.

See [RX_CLIENT_VERIFICATION.md](RX_CLIENT_VERIFICATION.md),
[RX_DESKTOP.md](RX_DESKTOP.md) and [VERIFICATION.md](VERIFICATION.md).
The [contact publication verification](CONTACT_PUBLICATION_VERIFICATION.md)
records 42 passing owner tests and replay of 23 publication plus six independent
classification exports through the actual Rust projector. They include row
absence, stale classification becoming Unknown, band/mode differences and
paper/LoTW/eQSL confirmation cases. This is worker-to-projector evidence, not
installed HTTP/UI propagation. Canonical xLog is now at
`16cac86739274073ab2f8000a3916b4e37bc57a5`, version 1.6.27. Its owner reports
portable/installer packages, installation and installed qualification passed,
with earlier failed guard receipts preserved. Current owner documentation is
`7789c569`; paired endpoint and station acceptance remain separate.
RX evidence includes 58 SDK tests, 40 distinct companion UI tests, TypeScript
and Vite builds, 14 browser viewport/theme cases, 14 pure DSP/assembler tests,
five native adapter tests including both modes at five rates, two actual owner
HTTP-to-native-decoder tests, and three threaded worker/authority tests.
These sets cover different boundaries and are not a combined coverage claim.

## xLog coordination

The latest integrated xNexus application source is
`0d01248b0504ae77f6f122fdada1b21e4a61806b`. Full Windows frontend and native
all-targets checks passed. A generated FT4 signal also passed through the real
decoder and owner HTTP/validator path into classification, pending-operation
revocation, expiry and Stop. Its classifier is synthetic; the actual Swift
reply replay remains separate evidence. See
[decoded classification verification](DECODED_NEEDS_VERIFICATION.md) for exact
receipts. Updated Linux tooling fixtures passed all 25 cases and its cache seed
passed with unchanged donor hashes. The subsequent contract phase was held
before container creation at 6.35 GiB physical headroom, below its 7 GiB gate;
updated SDK/UI and desktop checks have not run. See
[the Linux evidence](../../scripts/xnexus/linux/PROVISIONING.md).

The updated Mac check reached 35% admission briefly, then fell below that
threshold before TypeScript started. No frontend/compiler child launched;
source, cache and earlier archive preservation passed, with no remaining owned
processes. Held receipt SHA-256:
`4019b0fda7a90e1db07d57500b42ae8fad3dd4f9eb3d08151d231de8bb459405`.
The latest Mac resource probe was 32%, below the 35% admission threshold, with
no new frontend/compiler child. These holds do not qualify the updated source
on either platform. Windows `.4` subsequently passed the separate optimized
build and artifact audit; physical DAX reception is still unqualified.

Earlier canonical xLog integrated the bridge and receive owner at
`5efffc1a23af68157e3c564eb2aa345256bea90a`. Validation includes 1,154 native
tests, 30 selected Swift tests, 33 focused bridge/audio checks, six actual
isolated FLEX metadata checks and three receive UI checks. Detailed receipts
are retained in the xLog audio worktree. No production device was captured.

Earlier, the “xlog work” chat accepted and installed xLog 1.6.26 from
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
tests passed. Full native owner validation then passed 1,164 tests with no
failures and two existing ignored tests. Canonical xLog integrated the source
and acceptance record at `f155bd2d9732bf8fbb32e89a6153804f284d3891`.
The earlier memory-capped compile failed allocation before tests and remains
separate evidence. The subsequent accepted installed xLog 1.6.27 includes this
fix; physical paired rotor/Stop qualification remains open.

The private Windows xLog setup passed host/worker identity and accessibility
checks, but capture/input geometry failed and keyboard navigation did not move
focus. That sequence remains incomplete, with evidence retained; production
xLog was untouched. That historical failure remains preserved. The operator
has since confirmed the desktop is unlocked and available; new private Nexus
UI acceptance is in progress with process/isolation checks before observation.

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
The original uncommitted merge and earlier rehearsal remain preserved; no upgrade
was promoted and the upstream base remains `c88740a94`.
Source review found an incoming serial-port alias mismatch that can incorrectly
clear a stuck-CW warning in independent mode. A separate review branch at
`50a5f1840` now repairs it: two added assertions failed before the fix and all
19 targeted real-library fake-port tests passed after it. Full incoming update
gates remain pending; see [the upstream review](UPSTREAM_REVIEW_2026-11-14.md).

Exact `.3` Linux validation passed 58 SDK, eight policy and 40 UI tests plus
TypeScript/Vite, followed by an offline Rust 1.93.1 all-targets desktop check
in 19m36s. All 5,472 source entries matched before and after. The reviewed
reduced profile used 4 GiB RAM, no extra swap, one Cargo/CMake job and no network;
host memory floors remained satisfied. The container exited. Original Node
heap failure and earlier admission holds remain preserved. See
[the Linux evidence](../../scripts/xnexus/linux/PROVISIONING.md) for exact receipts
and limitations.

## Operational boundaries

The operator confirmed FLEX slice A → DAX Audio RX 1. Verify that explicit
selection against current FLEX metadata and the active stable endpoint before
capture; channel 1 is not an inferred default. An earlier read-only active-input
probe found ten active inputs and none DAX, although DAX devices were registered.
No DAX utility or stream was started by that probe. Live reception remains pending.

Receive timing reports host endpoint clock calibration, not a measured global
UTC accuracy or endpoint-to-RF delay. Both latter values remain explicitly
unknown. Generated-message tests do not establish physical reception.
Decoded-row classification is integrated under
[the agreed contract](NEEDS_QUERY_CONTRACT.md). One bounded
read mailbox attaches only to an existing ordinary xLog cluster poll; it creates
no query writer, poller or station connection. Owner tests, 80 SDK checks, replay
of 40 actual Swift replies and 19 native checks passed. The full 55-test companion
UI set, TypeScript, production bundle and 14 private-browser cases also passed.
Full integrated Windows all-targets checking and the `.4` optimized build/artifact
audit passed. Installed/native paired acceptance remains pending. See
[the verification record](DECODED_NEEDS_VERIFICATION.md).
The needed-station board continues to use xLog's classified cluster observations.
Transmission is unavailable in companion mode.

The native outbox retains up to 10,000 operations, including terminal receipts.
An archive/export flow remains a long-term operational item.

Full Windows GNU all-targets compilation passed on exact RX runtime `e3ae1534`
with pinned Rust 1.93.1 and locked dependencies. The isolated receipt verified
original dependency hashes and unchanged runtime source. The Mac core `.1`
archive passed the supplemental seven-file Mach-O audit. Native checks
verified visible identity, readable windows, four ordinary Quits and recovery
of an unqueued synthetic draft. That persistence check did not establish
cross-profile isolation; `.2` subsequently exposed the shared default store.
Theme/scale are session-only. Historical `.1` CLI path provenance remains incomplete:
the downloaded official CLI is not proven to have been invoked.

The completed Mac `.2` build positively used a locally cargo-installed Tauri 2.11.5
CLI, whose path/hash and installation metadata are retained, with application
compiler Rust 1.93.1. A supplemental record corrects the original downloaded-CLI
assumption. The future build helper now invokes the verified absolute CLI path.
These are local unsigned candidates, with no Developer ID/notarization claim.
See [the detailed checkpoint](QUALIFICATION_2026-11-14.md) for artifact hashes,
qualification limits and preserved failed attempts.

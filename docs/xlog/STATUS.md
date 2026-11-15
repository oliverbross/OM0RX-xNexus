# Execution ledger — 15 November 2026

The integration remains in qualification. Source, synthetic tests, packaged
binaries and physical station acceptance are separate evidence levels.

[Candidate 1.24.1-xlog.4](CANDIDATE_1.24.1-xlog.4.md) has a completed Windows
build, static artifact audit and independent retention from clean canonical
`ae35fc6435f861e38029049299833afe57de4885`. The [disposable install/uninstall](WINDOWS_INSTALLER_1.24.1-xlog.4.md)
passed, followed by native A→B→A profile/lifecycle checks and permanent current-user
installation. A subsequent explicit installed launch passed; both applications
were left open. The operator subsequently completed live native pairing, but
receive failed an owner clock guard and RX sharing was stopped. The first owner
fix shipped in xLog 1.6.28. The operator confirmed a fresh pairing/sharing/FT8
attempt, but at 04:07:59 UTC it failed a distinct 50 ms uncertainty guard
(50,459 microseconds). No decoded rows/needs were observed or receive soak started.
Sharing was stopped. The reviewed recovery correction was included in
clean, pushed xLog 1.6.29 candidate `1ff361bfa4f0d1f92c1f1f0323f9dcd166f1da44`;
its optimized native compile completed in 18m31s with root exit 0. A retained
MSVC VCTIP helper caused lifecycle exit 125; cleanup confirmed zero processes.
That candidate was superseded and never packaged or installed because of a
separately confirmed [retained-cluster reply-shape defect](RETAINED_CLUSTER_INCIDENT_2026-11-15.md).
The host/projector read an extra `response.result` level, guaranteeing empty
companion cluster output even for a nonempty retained owner payload. The
actual producer regression reproduced the missing classification/logbook revisions,
then passed after correction `4025ee6f057e811343a506ea131bba9acd1ea312`.
Related 20 cluster/nine projection tests, 127 bridge tests and 29 unchanged-input
export replays passed. Clean, published xLog 1.6.30 candidate
`8795606bdbb5ac029287518a826f189d8873d117` combines both fixes and was packaged
and installed at 05:56:28 UTC, with 3,254 payload files verified and all 11 profile
files preserved. Private candidate and installed marked launches/ordinary closes
passed. Normal installed handoff passed 36.5624911 seconds/13 samples, host 6540
and worker 24672, diagnostics off and stable FLEX/DCU; xLog was left open.
New 1.6.30 pairing/receiving confirmation and live RX/needs remain pending. The earlier tests
are not retroactively reclassified as actual retained-owner wiring proof.
It remains unsigned. [The build record](WINDOWS_BUILD_1.24.1-xlog.4.md) binds the
actual tools, reviewed 12 GiB supervisor, artifacts and warning qualification.
The packaging freeze is released for reviewed documentation/tooling integration;
canonical `main` first integrated that evidence at `78e1c288`, with application/runtime
and packaged provenance still exact `ae35`. The earlier xLog 1.6.28 handoff passed
37.715 seconds/12 samples with diagnostics off; this historical result does not
replace the separately passed 1.6.30 installed handoff. Its [installation and handoff evidence](RETAINED_CLUSTER_INCIDENT_2026-11-15.md)
preserves the separate Host VCTIP lifecycle failure and reviewed qualification.
Installed xNexus paired with the real workspace/protocol 1 and displayed live
DIGU correctly. Two observations over 46 seconds showed the owner clock error,
zero decoded rows and no decoded-needs evidence. The operator stopped RX sharing.
An owner metadata-only probe identified 10–17 ms endpoint sample-clock residuals
against existing 2 ms guards. The reviewed owner correction and restart UI passed
16 Rust/four UI tests and independent origin-pair replay, then shipped in 1.6.28.
The fresh 1.6.28 attempt then failed the uncertainty bound. Owner fix
`9f75def1d41ec695165eca2cc016f2226b687651` now discards invalid intervals and
recovers with explicit new epochs, preserving raw timestamps and the 50 ms limit.
Its six-recoveries/rolling-60-seconds operational bound stops persistently failing
input. Native six-minute metadata observation and exact Buffer replay passed;
[cross-application recovery acceptance](../../integrations/xlog-paired/RX_RECOVERY_ACCEPTANCE.md)
also passed against unchanged `.4`: two five-client cases, 19 Worker tests and
generated FT4 recovery after actual declared-gap/timing discards. These results
do not qualify live receive/needs; the 1.6.29 candidate is now superseded.
Owner `stopped` state retains the companion subscription
and can resume on a new valid descriptor, so this is not a terminal UI-state claim.
Both live failures remain preserved; see [the incident and exact evidence](RX_CLOCK_INCIDENT_2026-11-15.md).
The project plan is not complete; live RX/needs, physical controls, TX and updated
platform/distribution qualification remain open.

At the earlier 1.6.27 handoff, the owner verified host PID 27440 and worker PID 17076,
10.136 MHz DIGU, receive slice A, TX idle and connected FLEX/rotor/KPA/KAT.
Settings input failed because coordinate geometry was unavailable; bounded
activation and Ctrl+,/Tab recovery did not change the focused pane, so input
stopped. The operator subsequently enabled xLog companion pairing manually.
Fresh owner accessibility observations confirmed a localhost endpoint and Revoke
button, plus DAX Audio RX 1 at 48,000 Hz/two channels. Current slice A selection
and capture had not been started at that observation.

On 15 November at 02:07:37 UTC, installed xNexus PID 19740 and WebView browser
PID 20148 passed normal process/profile verification. Accessibility showed `.4`
at `ae35`, with a blank unpaired view. One pairing-button click failed because
input geometry was unavailable, and no blind retries followed. Both applications
were left open. The operator was asked to select the native pairing file, confirm
current A/RX1 and enable sharing in xLog, then choose FT8 and Start receiving in
xNexus. The operator then completed those steps, establishing the narrow pairing
pass and subsequent clock failure recorded above. No credential/path is recorded
here. The original automation failure remains separate from that live RX failure.

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
| M2 | Separate companion startup; no local station services; `.4` Windows A→B→A private profiles and ordinary lifecycle passed | Final candidate native pairing and installed station lifecycle |
| M3 | xLog-authoritative call/DXCC needs, band/world maps, selected-call history; owner/SDK/native/UI tests and integrated Windows all-targets check passed; `.4` Windows artifact audited | Paired UI; installed classification propagation; richer confirmation/award policy evidence |
| M4 | Owner-mediated tune/point and independent Stop; connection-bound Stop fix merged after 95 focused and 1,164 full native tests; current xLog 1.6.30 installation/handoff passed | Paired UI and physical control qualification |
| M5 | Durable outbox and atomic owner write/receipt; restart, replay and edit-conflict tests | Paired native GUI acceptance; queue archive/export design |
| M6 | FLEX slice A → DAX Audio RX 1 selected; bounded owner PCM, client, FIR/period assembler, threaded native FT8/FT4 decoder and receive UI; generated FT4 to needs chain passed | Confirm live mapping/endpoint; active reception and clock/route qualification; TX implementation and supervised qualification |
| M7 | Distinct identity, repaired profile construction, reviewed installer guards; `.4` Windows build, audit, retention, A→B→A, disposable install/uninstall and permanent installation passed | Paired installed station acceptance |
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
installed HTTP/UI propagation. Installed xLog is now at
`8795606bdbb5ac029287518a826f189d8873d117`, version 1.6.30. Its owner reports
portable/installer packages, installation and installed qualification passed,
with earlier failed guard receipts preserved. Current owner documentation is
`a0be04b450e198e0e0c41d6299b004e9e88cfc30`; repeat live RX/needs and station acceptance remain separate.
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

An earlier updated Mac check reached 35% admission briefly, then fell below that
threshold before TypeScript started. No frontend/compiler child launched;
source, cache and earlier archive preservation passed, with no remaining owned
processes. Held receipt SHA-256:
`4019b0fda7a90e1db07d57500b42ae8fad3dd4f9eb3d08151d231de8bb459405`.
A subsequent exact `0d01248b` run passed TypeScript, then held before Vite because
the next step missed its 35% admission threshold. Initial availability was 39%,
37% at TypeScript admission, and the 29% runtime minimum stayed above the 20%
floor. No Vite/native child ran; runner exit 1, no owned process group remained,
and source/cache/archive preservation passed. Receipt on the Mac:
`/Users/oliver/Projects/OM0RX-xNexus-integrated-needs-check/target/integrated-check-20261115T022950Z-cb7a009f/receipt.json`,
SHA-256 `7ba5cc2afa5e2c271220d017afa9e319a0521665617544e795b0fed84a1a7d1e`.
No retry, package or UI launch followed. TypeScript success does not qualify
the remaining updated Mac or Linux gates. Windows `.4` passed its separate
optimized build/artifact audit; physical DAX decoding is still unqualified.

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
UI acceptance has now passed A→B→A private profile/lifecycle checks, with exact
process/isolation checks before accessibility observation. Graphics capture
timed out; no screenshot qualification is claimed. No pairing or QSO submission
was part of that check. Permanent current-user installation also passed and left
the application closed at that stage. The subsequent explicit installed launch
passed and left it open, still unpaired. See [the exact receipts](WINDOWS_INSTALLER_1.24.1-xlog.4.md).

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

A separate source-confirmed UI backlog item remains: RxPanel's successful status
poll does not clear an earlier poll-error alert. The [incident record](RX_CLOCK_INCIDENT_2026-11-15.md)
distinguishes that obsolete-alert risk from the fresh owner timing failure.

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

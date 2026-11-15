# Execution ledger — 15 November 2026

The integration remains in qualification. Source, synthetic tests, packaged
binaries and physical station acceptance are separate evidence levels.

## Current source checkpoint

Windows [`.5` built, passed static audit and was independently retained](WINDOWS_BUILD_1.24.1-xlog.5.md)
from exact clean `75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`. It is **internal
testing only**, not native-qualified or installed. The user requires the original
Nexus operating interface; the diagnostic companion dashboard is not the final
product and `.5` will not be installed over it. M2 must preserve the original
operating UI with xLog services underneath. M6 TX is required and remains
unfinished software work, not merely waiting for a hardware test.

The original-interface host is committed at
`ceb0ef61bb1360d8979ddb8bb329a06ad5e9c4f4`; checkpoint `b84610861` also includes
the original Spots view, the read-only original contact-card surface, a strict
contact-detail codec, bounded SDK/native/frontend readers, the original global
rotor strip and updated notice tooling. The tested rotor helper,
offline waveform exporter and native history wiring remain included. The production
xLog entry now mounts the original Nexus cockpit. These changes do not change
the installed application or qualify a paired native station candidate:

| Source change | Evidence and remaining boundary |
|---|---|
| Scoped original log readers (`453bdc7c3`) | 18 focused tests and integrated TypeScript/Vite passed. Changing an owner source clears retained answers; unsupported owner reads never fall back to the native log. |
| [Saved station identity client](STATION_IDENTITY_CLIENT_VALIDATION.md) (`cccf2360b`) | 85 SDK tests, actual owner-produced synthetic identity replay, pinned Clippy and integrated TypeScript/Vite passed. Saved call/grid are display facts, not transmit authority. |
| [Native waterfall](WATERFALL_SOURCE_DRAFT.md) (`c686560bb`) | Six SDK audio, nine assembler and 34 native Worker tests passed. Real existing PCM supplies the FFT; gap/expiry/context changes discard old frames. Subsequent native all-target checking and original full-host waterfall presentation passed; linked native tests remain a separate gate. |
| [Pure FT8/FT4 period preparation](../../integrations/xlog-tx-prepare/VALIDATION.md) (`5e7f3e47f`) | Six actual encoder/waveform reference and boundary tests, formatting and offline locked manifest checks passed. This deliberately unwired module covers strict standard type-1 messages only; it does not provide sequencing, scheduling, PTT or a TX endpoint. |
| [Original cockpit service connections](ORIGINAL_COCKPIT_SERVICES_DRAFT.md) (`795cfe955`) | 19 new tests, integrated TypeScript/Vite and 21 original CallCard browser cases passed. The closest upstream suites recorded 365 passing cases across a preserved partial timeout and completed follow-up; the broader 161-file run is not qualified. Original markup/layout remains; missing services cannot acquire a separate station owner. |
| Original recent-contacts pane (`ff59ec067`) | Nine unit tests, integrated TypeScript/Vite and 12 real-pane browser cases passed. Distinct native positive controls, the original responsive viewport hook, visible callsigns and row clicks were checked. Explicit owner loading/unavailable states remain different from an empty log, without fabricated QSO frequency/confirmation fields. |
| [Complete owner history/recent read models](../../integrations/xlog/HISTORY_READ_VALIDATION.md) (`93358bb7a`) | 97 SDK tests, actual Swift history and identity producer replays, pinned Clippy and integrated TypeScript passed. Native connection wiring has a separate qualification gate. |
| [Original history panes](ORIGINAL_HISTORY_UI.md) (`984fbaadd`) | 84 focused card/reader regressions, eight actual Swift payload replay cases and both single-pane and original Classic-rail browser checks passed. Full counts and paper-QSL/LoTW policy remain distinct from limited detail rows. |
| [Current-context waterfall reader](SPECTRUM_READER.md) (`e75996693`) | 33 frontend tests with actual Rust fixtures and integrated TypeScript passed. Full-host browser checks also rendered a real frame and observed its lease-driven reset. |
| [Original default operating window](ORIGINAL_HOST_VALIDATION.md) (`ceb0ef61b`) | Original TopBar, ModeNav, Classic/Roster cockpit and waterfall/history panes are the production entry. Focused 34-, 449- and five-case gates, full integrated TypeScript, three-viewport/six-layout browser checks and actual production bundle graph proof passed. Groups overlap; these are not a unique-test total. No native installation or RF qualification follows. |
| [Rotor observation helper](ROTOR_CONTROL_OBSERVATION.md) (`21249e7c7`) | 31 focused tests and a separate strict TypeScript gate passed. The helper preserves command receipts separately from later owner snapshots; it is still unwired to Point dispatch. |
| [Original Spots view](ORIGINAL_SPOTS_DRAFT.md) (`0829c6f5b`) | 29 owner/hook cases, 50 existing native cases, 406 catalog checks, TypeScript/Vite and 12 browser cases passed. Real original components render synthetic owner observations; exact IDs/Hz, expiry, Unknown assessments, preserved cockpit state and global receive Stop were checked. Row selection does not tune or transmit. |
| [Original contact-card surface](ORIGINAL_CONTACT_CARD_SOURCE_DRAFT.md) (`a13077213`) | 70 component cases plus 13 corrected by-ID cases, 435 catalog checks, three actual Swift producer replays, final strict TypeScript and seven browser cases passed. The original native card remains a positive control. This initial surface is read-only; the readers and supported editor below are subsequent source changes. Production host mounting remains in qualification. |
| [Strict contact-detail codec](../../integrations/xlog/fixtures/CONTACT_DETAIL_PROVENANCE.md) (`aa23015ab`) | 15 focused tests, including the three actual Swift producer states, and all-target Clippy passed. Explicit nulls, raw confirmation provenance, UUID representation and complete edit bases remain distinct. This codec gate alone does not qualify transport or UI; the other 100 SDK tests were filtered. |
| [Bounded SDK detail client](../../integrations/xlog/CONTACT_DETAIL_READ_VALIDATION.md) (`aa237af25`) | 37 codec/client cases and all-target Clippy passed. Exact by-ID transport, independent request fences and retained serialization leases were checked; no live paired endpoint follows. Earlier Windows fixture and lint failures remain preserved. |
| [Native detail lane](../../integrations/xlog/NATIVE_CONTACT_DETAIL_VALIDATION.md) (`2103df7c6`, record `9103d9c2b`) | 13 focused runtime cases passed. A separate actual integrated-shell check passed fresh normal and test targets with zero test execution, natural retirement and unchanged retained evidence. The initial three-document checkout-byte failure and narrow Git line-ending verification remain distinct records. |
| [Frontend selected-contact reader](CONTACT_READER_VALIDATION.md) (`77950e35f`) | 90 selected cases (47 contact, 35 history, eight actual history producer replays), full TypeScript/Vite and seven original-card browser cases passed. The native-only reader preserves request-start deadlines, repeated-evidence high-water marks through availability loss, worker retirement and immutable facts. It is not yet mounted from Recent/Recall. |
| [Original global rotor strip](OWNER_ROTOR_STRIP.md) (`26d2d6841`) | 24 owner, 12 native and 406 catalog cases, full TypeScript/Vite and nine original-host browser cases passed. Exact retained Stop remains available through telemetry expiry, pending work and poll failure; actual connection/target replacement retires its authority. No physical movement is claimed. |
| [Point poll and settlement tracking](POINT_POLL_EVIDENCE_DRAFT.md) (`5b9429500`) | 64 focused cases, nine original-host browser cases and final full TypeScript/Vite passed. All 407 catalog assertions passed; the original aggregate runner failed its obsolete 406-count expectation and remains preserved with a separate read-only qualification record. A narrow request-ID type correction produced byte-identical emitted JavaScript. The explicit dispatch entry is not yet mounted to a Point control; no physical movement is claimed. |
| [Supported original contact editor](ORIGINAL_CONTACT_EDITOR_SOURCE_DRAFT.md) (`b84610861`) | 85 follow-up cases across controller/editor/storage/recovery, 436 catalog cases, full strict TypeScript and eight actual-card browser cases passed. Six supported owner fields reuse the original form; Queue, Submit and Applied remain separate. Multiline Enter, immutable operation recovery and legacy Clear were checked. Earlier product/fixture failures remain preserved. The controller and original card are qualified separately from the pending host mount. |
| [Locked notices and exact resource coverage](SOURCE_DELIVERY_GAPS.md) (`e1799c3f1`) | Offline generation/check covers 531 normal-edge desktop crates; 13 Node and nine Python controls passed. Historical notice mappings were checked without modifying retained artifacts. Complete corresponding-source/model provenance and platform release gates remain open. |

The actual original window now passes isolated frontend/browser qualification.
It preserves nullable owner facts, explicit receive controls and original decode
versus Stations-selection gestures. Four failed browser probes remain preserved;
their corrections changed fixture assumptions/readiness, not product behavior or
expiry limits. The real production bundle includes OriginalHost and excludes the
old dashboard from that entry. Native tests, a new version with build provenance,
private candidate checks and paired acceptance are still required before installation.
The native all-target check passed with fresh normal and test application
targets (receipt `69E860819DF1EDBED291B1AEEA3755842BB77D7BBD5385585D26E0BF602EF364`).
The linked test build exceeded its original 4 GiB cap before executing tests;
that failed result is preserved. A separately reviewed 8 GiB test profile with
14/14 GiB admission and 4/4 GiB runtime reserves then passed all 12 selected
connection/history tests. It exited naturally with zero remaining processes and
no limit notifications (receipt
`0CD14FFFFD904CBFA34A2D56F969ED9725E502C5ADE9FA149AAAFBF33968FC90`).
The [native evidence record](../../integrations/xlog/NATIVE_HISTORY_READ_VALIDATION.md)
preserves the earlier resource-tool, sandbox and memory failures. This remains
source/test qualification; it does not constitute a packaged native-window pass.

The [offline waveform corpus](../../integrations/xlog-tx-prepare/examples/README.md)
contains 14 actual CQ/grid/report/closing periods across FT8 and FT4. Independent
byte/manifest checks passed, and xLog's actual validator accepted every period
without transforming the bytes. Its cached confirmation run retired normally
with zero remaining processes; the earlier retained-compiler-helper failure is
preserved. xLog committed this offline validator foundation at `cf30734b` with
12 synthetic tests and the actual producer replay. No output route, scheduler,
transmit endpoint or hardware backend follows from content validation.

The catalog repair at `6b40a539b` passed 441 checks. Combining the Spots and
contact-card changes required retaining both independent catalog additions;
the resulting source passed all 407 selected catalog checks before promotion.
None of these commits bumps a version, packages an application or changes an
installed binary. The additional browser failures remain recorded as fixture
corrections, without relaxing product geometry or lease limits.

xLog's by-ID producer is committed at `635f754d`. Its actual
found/deleted/not-found fixtures preserve nullable frequency, multiline comment,
stored mode/submode and authoritative paper QSL method. The subsequent passive
read lifecycle correction at `7fe76148` passed five focused host tests in a clean
cached confirmation: read timeouts preserve the exact worker while mutations
retain their existing retirement policy. The earlier retained VCTIP failures
remain failures. The authenticated endpoint is now committed on the xLog side;
canonical `70599ecf` also contains the escaped-comment wire-size correction with
seven actual mutation/replay tests. The bounded client, native lane and frontend
reader are qualified source as recorded above. Installed xLog does not gain this
capability from source commits.

The actual integrated native check at `2103df7c6` passed under the unchanged
4 GiB profile, with fresh normal and test metadata targets, zero remaining
processes and no forced cleanup. It executed zero tests; the 13 focused native
lane tests are separate runtime evidence. Receipt:
`FE1FADADF0538D0F3928CD15FD3C011A1D21AD6D1F34E1B711441C75762BE98E`.
The near-cap peak and retained source/tool/cache inventories are recorded in the
native validation document, without implying a linked test or packaged pass.

M5 next mounts the qualified by-ID read and supported original editor from the
actual Recent/Recall gestures. That host work remains uncommitted and untested:
it must check synchronous current read/write authority, respect close/replacement
guards and retain the durable outbox's immutable operation identity. A draft may
remain editable during a refresh without granting stale submission authority;
known revision changes must preserve the draft and require explicit resolution.
The compact history projection alone does not establish full contact or
confirmation completeness. Point tracking is committed, but its UI dispatch,
bearing transport and physical outcome qualification remain separate work.

xLog's passive bearing producer at `9d7555d5` passed its actual Core build and
13 Swift tests. Its four-state exported fixture is retained with SHA-256
`307BD860A42BC5A3191FA0F421BFCA32AB0C27CC44DF0843840AE0F57B43FE8D`.
The endpoint is still in qualification and is not installed. The xNexus pure
body codec is uncommitted qualification work: its initial run passed 11 cases
and failed the actual replay's JSON integer-versus-float representation
comparison. The unchanged producer body decoded successfully; a test-only
correction preserves all integer identities and compares only declared
floating-point fields numerically. The corrected, separate run passed all 12
cases and SDK all-target Clippy with unchanged producer bytes, natural exit and
zero descendants; receipt SHA-256 is
`BDF723495350690E04683A0ADE79E419828480F7B4100AC32954DD056FEA3CCC`.
This pure body-codec evidence does not qualify the endpoint or client transport.

The [TX sequence implementation plan](TX_SEQUENCE_ADAPTER_PLAN.md) is committed
at `4d21abcf7`. An isolated, not-yet-committed offline adapter now reuses the
actual Nexus Station state machine, retains prepared bytes and accepts only an
explicit fake actual-start event. Its 22 tests and the existing six waveform
goldens passed fresh compilation of the actual adapter/preparer modules against
source-equivalent retained dependencies. The bounded Job exited naturally with
zero descendants; receipt SHA-256 is
`0E11EAF98C21C20A04651556E82090843C87B57F90A74C49870586BF204D564E`.
Cargo/Clippy and actual-owner manifest/body replay are separate gates. It
provides no scheduler, output device,
transmit endpoint or RF permission. The actual manifest transfer, exact physical
ownership/release and original operating-control wiring remain unfinished.
M2 and M3–M8 remain open. No new package, version, RF test or installation is
claimed by this source checkpoint.

Installed xNexus remains `.4` at `ae35fc643`. The xLog owner reports `1.6.31`
at `a00697f5458c64cf806e4c9647fcd133fc30391b` packaged, installed and privately
checked; its subsequent normal handoff also passed. Both applications were
closed for the serial package build; xLog's later normal handoff left it open.
That is historical lifecycle evidence, not a current process observation.
No current live RX/needs pass is claimed.
The following diagnostic history preceded that owner installation.
The native metadata diagnostic reproduced an oversized UTC/QPC correlation
bracket. Owner correction `fa312941188991c6cf5a1a7e7c6bf8241c450647`
adds bounded correlation retries without widening the 1 ms limit and fixes
COM cleanup on open failure. A later clock-retry capture still failed with
generic E_FAIL after 2.469 seconds/87 records; that run did not identify its
predicate. A subsequent instrumented run failed after 19.422 seconds/752 records:
only the packet-age predicate failed (543.01265 ms, with DATA_DISCONTINUITY).
Its first correlation attempt was valid at 0.5 microseconds. Stale audio was
correctly rejected; this does not prove the installed failure's cause. Both
probes ran BelowNormal, unlike the installed Normal-priority owner; a separate
Normal-priority diagnostic then passed 360,000 ms with 14,388 metadata records:
14,382 publishable, one startup discard and five declared gaps, with no fatal
result. Three publishable packets used the second clock attempt. Root/final
results were zero and cleanup was confirmed. Exact production Buffer replay also
passed: 14,382 published packets, five recovered gaps and zero timing discards.
Installed paired acceptance remains pending. No production limit changed.
See [the preserved diagnostic sequence](RX_CLOCK_INCIDENT_2026-11-15.md).

Owner classification-cache correction
`979b7ff748992c73ee99571eabb65b0a5e68a20e` passed 40 focused Windows host tests
(nine needs, 20 cluster, nine projection and two worker-routing tests). The
coordinated Nexus scheduler/cache correction is integrated at
`9383d286c480fd94820cf63a6651ef6d6f717076`: bounded pending groups preserve query
identity across transient misses, and cache admission matches positive serialized
lifetime. All 23 focused native tests passed; the independent-phase 179/200-row
matrix had already passed. See [the focused evidence](NEEDS_PENDING_RETRY.md).
The [actual owner cache/mailbox → SDK → generated FT4 native Worker chain](../../integrations/xlog-paired/NEEDS_CACHE_CHAIN.md)
also passed using those exact product sources and owner `979b7ff7`: four queries,
two poll classifications, same-ID 503-to-Known recovery with original timestamps
and TTL, Unknown with continued PCM, fresh Known, Stop and normal shutdown.
A separately reviewed Normal-priority replay used identical compiled bytes after
the initial BelowNormal attempt failed its unchanged synthetic pacing guard.
Both attempts are preserved; the passing Job exited 0 with zero descendants.
Classification was synthetic, not a Swift logbook; neither this chain nor owner
tests qualify live decoded-needs operation. They did not change installed `.4`;
the subsequent owner `.31` installation is a separate evidence boundary.

Two independent UI fixes are now on canonical main: exact band-map row-to-owner
identity (`30ee3e98a9aab1889835186146ff093ce4a290eb`) and separate polling/action
error state (`4f4d431c88f0f1481084a20c71aad09721a9b9aa`). Both had failing-first
regressions; 26 focused tests then passed (five band-map, six receive-panel,
15 needs-lease tests), with no remaining test processes. The receipt SHA-256 is
`FE901636C2E26F98656FFEA5B0C630CD402A16700F90D286F691D8FD345BC947`.
The integrated `.5` frontend/browser gates and Windows package now passed;
installed `.4` has not changed. Package success does not qualify the diagnostic
dashboard as the requested original Nexus interface.

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
The operator completed new 1.6.30 pairing/receiving. Sixteen native observations
from 06:07:09.881 to 06:11:38.140 UTC showed actual new FT8 periods, rows increasing
56→123→179→200, and 64→72 live cluster Needed rows with scores/reasons. Before
six minutes, `RX device/timestamp discontinuity (0x80004005); restart sharing.`
cleared the route/rows. The healthy-RX soak stopped immediately, with no input;
apps were left open. Decoded-call needs showed Awaiting/unavailable/expired,
never qualified Needed/Worked. The later metadata diagnostic reproduced one
clock-bracket failure path, but its correction has not passed live qualification
and a subsequent generic failure remains under investigation. The earlier tests
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
All three live failures remain preserved; see [the incident and exact evidence](RX_CLOCK_INCIDENT_2026-11-15.md).
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
| M2 | Separate companion startup; no local station services; `.4` Windows A→B→A private profiles and ordinary lifecycle passed | Restore the original Nexus operating interface over xLog services; diagnostic dashboard is not the final product; qualify native pairing/lifecycle |
| M3 | xLog-authoritative call/DXCC needs, band/world maps, selected-call history; owner/SDK/native/UI tests and generated cache/FT4 chain passed; `.5` internal Windows artifact audited | Original operating UI integration; installed classification propagation; richer confirmation/award policy evidence |
| M4 | Owner-mediated tune/point and independent Stop; connection-bound Stop fix merged after 95 focused and 1,164 full native tests; owner reports 1.6.31 installation/private checks and normal handoff passed | Paired UI and physical control qualification |
| M5 | Durable outbox and atomic owner write/receipt; restart, replay and edit-conflict tests | Paired native GUI acceptance; queue archive/export design |
| M6 | FLEX slice A → DAX Audio RX 1 selected; bounded owner PCM, client, FIR/period assembler, threaded native FT8/FT4 decoder and receive UI; generated FT4 to needs chain passed | Confirm live mapping/endpoint; active reception and clock/route qualification; TX implementation and supervised qualification |
| M7 | Distinct identity, repaired profile construction, reviewed installer guards; `.4` installed acceptance and `.5` internal-only build/audit/retention passed | New original-interface candidate and paired installed station acceptance; no `.5` upgrade |
| M8 | `.4`/`.5` Windows artifacts audited; Mac .3 A→B→A isolation and exact .3 Linux contract/desktop checks passed | Current notices/resource coverage, updated Linux/Mac packages, paired UI, native lifecycle, signing and distribution qualification |

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

The integrated decoded-needs baseline is
`0d01248b0504ae77f6f122fdada1b21e4a61806b`. Full Windows frontend and native
all-targets checks passed on that source; the newer UI source checkpoint above
has only its stated focused checks. A generated FT4 signal also passed through the real
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
No DAX utility or stream was started by that probe. Later operator-started live reception produced decodes, but sustained receive qualification remains open after the recorded failures.

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

The obsolete RxPanel polling-alert defect is fixed in source at `4f4d431c`:
successful polling clears only its own error and preserves action errors.
Installed `.4` still contains the old behavior. The
[incident record](RX_CLOCK_INCIDENT_2026-11-15.md) distinguishes that alert from
fresh owner timing failures; the fix does not repair capture timing.

## Remaining software work after the immediate RX/needs corrections

The remaining plan is not only physical acceptance. Prioritized independent
software slices and their gates are recorded in
[the delivery plan](PROJECT_PLAN.md#next-independent-software-slices).
They include accepted-control receipt reconciliation, durable outbox archive/export,
actual-owner native logbook lifecycle coverage, richer confirmation/award evidence,
propagation-view reuse and offline TX ownership/scheduling implementation.
Updated platform builds, source/license delivery and upstream promotion also
remain open. Supervised radio/rotator movement, actual log-change propagation,
TX commissioning and final installed qualification are separate gates; none is
inferred from the passing component tests.

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

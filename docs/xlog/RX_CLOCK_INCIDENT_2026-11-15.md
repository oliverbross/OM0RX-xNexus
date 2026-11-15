# Live receive clock failure — 15 November 2026

The operator completed native pairing, confirmed slice A → DAX Audio RX 1,
enabled owner sharing and started FT8 receive in installed xNexus `.4`.
The unchanged application at `ae35fc6435f861e38029049299833afe57de4885` paired
with the real xLog workspace using protocol 1 and displayed the live DIGU
observation correctly. This qualifies that narrow live pairing/display path.

Receive then reported the owner's clock-validation error. Two observations
46 seconds apart showed zero decoded rows and no decoded-needs evidence.
The operator stopped RX sharing at the end of that failed attempt. Successful pairing does
not qualify live decoding, classification, physical controls or transmission.

Private receipt under the xLog workspace:
`.build/xnexus-installer-acceptance/installed-normal-launch-cd403fcc627d4b9195fbb46fa6f231fb/rx-live-failure.json`,
SHA-256 `86E217F462E92D657C97FF9EC0055FA28A54E02528559AFD482E22C31AA08865`.
The receipt remains failure evidence, not a successful RX qualification.
No pairing secret, workspace identifier, device identifier or station call is
included in this repository record.

The xLog owner subsequently reported a 30-second metadata-only probe covering
1,198 packets with contiguous frame positions and no PCM capture. WASAPI
GetBuffer QPC versus nominal-frame residuals were approximately 10–17 ms,
with a 17.3869 ms maximum. UTC-to-QPC calibration stayed below 0.23 ms.
The timestamp units were correct; the existing 2 ms sample-clock guards
rejected the observed endpoint timing.

Owner, root and independent review agreed a narrow owner-side correction:
preserve raw timestamps and use the existing receiver's 50 ms contract, bounding
the range of exact wire UTC/frame residuals plus maximum calibration uncertainty
so the bound holds for any subscriber origin. Per-packet wall-step, frame,
flags and liveness guards must remain. This is not permission to invent samples,
resample away timing evidence or claim measured global UTC/RF accuracy.

## Owner correction installed before the live repeat

xLog 1.6.28 now includes timing fix `389281bf` and restart UI change `b233a991`.
The owner reports 16 Rust and four UI tests passed. Independent replay checked
718,201 subscriber-origin pairs with a worst bound of 17,521 microseconds.
This supports the corrected admission calculation, not physical decoding.
No xNexus runtime/package change was made; `.4` remains exact `ae35`.

The installed owner source is `eb3cf7024f7355c16346282646c7d2d62fb6bb7c`,
with current documentation at `92d6547`. Under the xLog workspace's
`.build/releases/1.6.28/acceptance/`, verified receipts are:

| Evidence | SHA-256 | Scope |
|---|---|---|
| `installed-verification.json` | `34C091A585D009426CC53D47AF34B06078AEAEE1369B011F87CF54F61FF313FE` | 3,254 payload entries verified; 11 profile files preserved, including RX binding |
| `normal-handoff.json` | `A505524D364E63AC26FE10B49A1BEEABE74853BE7D58F3DD10464867EAF40420` | 37.715 seconds/12 samples; host 20200 and worker 13980; diagnostics off, left open |
| `candidate-six-minute.summary.json` | `8D0DAC82B2A79B9400D6952A687D7FE2642F00E378893DA7D1802BAA0C6E0BA3` | 132 samples, 1,569 replies, zero transport failures/worker retirements, ordinary close |

The exact owner installer SHA-256 is
`7a25a76bb4556b3c0e1cbae1a7a2a1dd7707c3a8d32c60c30472335d676a8ed6`.
The six-minute harness initially expected `reference_resolve` for a blank draft;
source review corrected that criterion to `reference_schedule`. The same retained
evidence contains four schedule calls, including one at 355.927 seconds. The
original harness failure remains preserved; no external download is claimed.
Host compilation passed in 11m18s, but its exact MSVC telemetry-helper guard
failure and separately reviewed narrow cleanup qualification remain recorded.
Worker, Portable and Installer jobs exited naturally and passed.

## Fresh 1.6.28 attempt failed the 50 ms uncertainty bound

After the operator confirmed all new pairing/sharing/FT8 steps, a fresh native
observation at 04:07:59 UTC on 15 November reported:

> Receive timestamp uncertainty exceeded 50 ms (50459 us); restart sharing.

The exact installed xNexus process was verified before observation. It was paired
with the real workspace but had zero decoded rows, no current audio route and no
needs evidence. No input was performed and the planned six-minute receive soak
was not started. This is a new, distinct 1.6.28 owner-bound failure, not reuse of
the earlier 1.6.27 observation and not successful live RX qualification.

Receipt beside the earlier failure is
`rx-1.6.28-new-attempt-uncertainty.json`, SHA-256
`FD536564A009BAA06D75FA52FE3FF53DC7FB6943EEED9B771619EDC48D4B797E`.
This prompted the bounded interval-recovery correction described below; it did
not qualify live RX or justify increasing the timestamp uncertainty limit.

Source review of [the native worker](../../src-tauri/src/xlog_rx_worker.rs)
confirms that an owner `stopped` audio state clears current route/authority and
waits while retaining the companion subscription. A new valid running descriptor
can resume reception automatically; this state is not itself a terminal stale UI.
Transport failures and explicit Stop have separate handling.

The 1.6.27 failure and 1.6.28 uncertainty failure both remain preserved. Repeat
live receive/needs acceptance before promoting either capability; the broader
project, TX and updated platform/distribution gates remain incomplete.

## Bounded recovery correction and compatibility acceptance

Owner fix `9f75def1d41ec695165eca2cc016f2226b687651` preserves raw timestamps
and the 50 ms any-origin bound. A startup outlier, an over-budget timing interval
or a declared audio gap is discarded rather than joined into continuous audio.
Recovery clears buffered authority and starts an explicit new epoch; subscribers
must discard old partial periods/facts and wait for a complete new period.
Physical clock, frame, flag, liveness, route and revocation checks remain.
At most six interval recoveries are admitted within a rolling 60 seconds;
exceeding that operational bound stops sharing. It is a bounded failure policy,
not a measured hardware tolerance or relaxation of the 50 ms contract.

The candidate native owner completed a six-minute metadata-only observation:
14,397 packets, one startup discard, 14,396 valid packets and zero failures.
Exact Buffer replay published all 14,396 valid packets without a timing discard.
That run contained no later gap. Separately, replay of the reproduced failing
trace's 14,396 rows published 14,393 packets, with one startup discard, one
timing discard and one declared gap. Both original live failures stay preserved.

[Cross-application recovery acceptance](../../integrations/xlog-paired/RX_RECOVERY_ACCEPTANCE.md)
passed against that exact owner fix and unchanged `.4` production source `ae35`:
two five-client SDK/HTTP cases, all 19 native Worker tests, and a paced generated
FT4 chain through actual owner Buffer/HTTP, SDK, assembler and decoder. Declared
gap and timing discard each cleared rows/needs; a complete new-epoch period
decoded again with fresh synthetic owner needs. The 147.5-second guarded run
peaked at 850,972,672 bytes and exited naturally with zero descendants. All 32
recorded source hashes stayed unchanged. This is synthetic compatibility evidence,
not live DAX, actual logbook classification or installed GUI qualification.

The clean, pushed owner candidate is xLog 1.6.29 at
`1ff361bfa4f0d1f92c1f1f0323f9dcd166f1da44`. Optimized native compilation completed
in 18m31s with root exit 0, but a retained VCTIP helper caused lifecycle exit 125;
cleanup confirmed zero processes. The candidate is superseded and was never
packaged or installed. At that stage, installed 1.6.28 was the failed live candidate
with sharing stopped. The [cluster correction](RETAINED_CLUSTER_INCIDENT_2026-11-15.md)
subsequently passed actual producer regression, related suites and unchanged-input
replay. Both fixes are in clean, published xLog 1.6.30 candidate
`8795606bdbb5ac029287518a826f189d8873d117`, implementation
`4025ee6f057e811343a506ea131bba9acd1ea312`. Package, installation and normal
handoff subsequently passed, with [exact receipts](RETAINED_CLUSTER_INCIDENT_2026-11-15.md).
The subsequent live 1.6.30 attempt is recorded below; the prior installed 1.6.28
failures stay preserved.
No xNexus runtime edit, version change or rebuild is required by the
tested recovery compatibility result.

## Live 1.6.30 decoded FT8, then failed before six minutes

After the operator confirmed new 1.6.30 pairing/sharing/FT8 steps, 16 native
accessibility samples spanned `2026-11-15T06:07:09.881Z` through
`2026-11-15T06:11:38.140Z`. Real FT8 rows increased from 56 to 123 to 179 and
then the 200-row display limit, with newly decoded periods. The observed route
was slice A/DAX RX 1, 10.136 MHz DIGU, 48 kHz stereo, selected input channel 1.
The live cluster also displayed 64→72 Needed rows with owner scores/reasons.
These are concrete live decoding/cluster observations, not a completed soak.

Before six minutes, the owner reported:

> RX device/timestamp discontinuity (0x80004005); restart sharing.

Rows and route cleared. The native healthy-receive soak stopped immediately;
both applications were left open and no input was performed. A separate owner
process/socket observer may finish, but cannot establish uninterrupted RX.
The generic live E_FAIL alone does not identify its predicate. Subsequent
diagnostic evidence and the separate corrections are recorded below; none
retroactively qualifies this failed installed attempt.

Decoded-call needs also remain unqualified. Twelve fresh decoded rows initially
showed Awaiting, then lookup/classification unavailable or expired; no decoded
row was observed with Needed/Worked evidence. Cluster Needed rows are a separate
path and do not qualify decoded-call classification or natural RX recovery.

The private receipt is
`.build/xnexus-native-acceptance/live-rx-1.6.30-20261115.json`, SHA-256
`AE56584FF4F74CD91DACD440333C1EB924351E71A8F5CFA21BD5730FE54866C0`.
It records a failed live attempt. No station calls, workspace/device identifiers
or credentials are reproduced here. All prior failures and synthetic recovery
results retain their original scope; live RX stability and decoded needs remain open.

## Diagnostic clock correction and unresolved live stability

The exact installed-source metadata probe reproduced E_FAIL after 11.203 seconds.
Only predicate bit 64 failed: the UTC/QPC correlation bracket measured
21,567 × 100 ns, exceeding the existing 1 ms limit. Capture HRESULTs were S_OK,
packet size 1,200, flags zero and packet age 42.99 ms. This identifies a reproduced
failure path; it does not establish that every generic live E_FAIL has that cause.

Owner correction `fa312941188991c6cf5a1a7e7c6bf8241c450647` retries only a
positive oversized bracket, at most three times, using the actual accepted
attempt's clocks and the unchanged inclusive 1 ms limit. Failed/backward QPC
remains fatal. It also releases local COM interfaces before CoUninitialize on
open failure. The earlier diagnostic incorrectly passed CPAL's display identifier
to the raw endpoint API, exposing that cleanup bug; those probe access violations
are separate from the installed receive failure. Hardware-free native open/read
tests passed, including exact HRESULT preservation and balanced apartment cleanup.

A later clock-retry capture still failed after 2.469 seconds/87 metadata records
with generic E_FAIL. The next instrumented capture, `capture-20261115-065856`,
failed after 19.422 seconds/752 records with only mask 512: packet age
543.01265 ms exceeded the unchanged 250 ms age limit. DATA_DISCONTINUITY was set;
the first correlation attempt was valid at 0.5 microseconds. The reader correctly
rejected stale audio. Both probes used BelowNormal process priority, whereas
installed xLog runs Normal. No age, clock, packet-size or recovery limit was widened.

The Normal-priority repeat, `capture-20261115-070404`, passed 360,000 ms with
14,388 metadata records: 14,382 publishable, one startup discard and five declared
gap results. Three publishable packets used the second clock attempt; there was
no fatal result. The actual probe priority was verified, root/final results were
zero and cleanup was confirmed. Trace SHA-256:
`97EC6DB6B913159911D20F348D303275EA619F8C00D6772E15CBFB6DE53A48FA`.
This qualifies that native diagnostic capture only. Replay through the exact
production Buffer and installed paired acceptance remain pending. All original
failed receipts remain preserved; no `.31`/`.5` installed acceptance is claimed.

## Decoded-needs correction remains in qualification

Owner `979b7ff748992c73ee99571eabb65b0a5e68a20e` makes classification completion
nonblocking across existing cluster polls, preserving original result age and
worker/workspace/authority/mutation fencing. Its complete Windows host compiled
and 40 focused tests passed. This changes neither the SDK's one-second HTTP timeout
nor two-second evidence TTL and starts no new station poller or connection.

The coordinated Nexus native scheduler/cache change is still uncommitted. Tests
found that cache freshness could remain true for a submillisecond interval after
the serialized remaining lifetime had rounded to zero. The reviewed correction
aligns freshness with a positive serialized lifetime; native validation remains
pending. Installed `.4`/xLog `1.6.30` are unchanged. A passing owner fixture does
not qualify decoded-call classification; both corrected candidates and paired
acceptance are still required.

## Polling alert fixed in source, not yet installed

[RxPanel](../../ui/src/xlog/RxPanel.tsx) previously retained a status-poll error
after successful refresh and could overwrite an action error with a poll failure.
Commit `4f4d431c88f0f1481084a20c71aad09721a9b9aa` separates those channels:
successful status refresh clears its polling error while preserving action errors.
Failure-then-success and action-error persistence regressions failed before the
fix and passed afterward. Together with the independent band-map identity fix,
26 focused UI tests passed with no retained processes. Installed `.4` still has
the original behavior; integrated frontend/browser and new candidate gates remain.
This UI correction is not an explanation or repair for fresh owner timing errors.

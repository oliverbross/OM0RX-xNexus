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
`1ff361bfa4f0d1f92c1f1f0323f9dcd166f1da44`. Its build is underway; package
completion and installation are not claimed. Installed 1.6.28 remains the failed
live candidate with sharing stopped. Live 1.6.29 receive/needs acceptance is
pending. No xNexus runtime edit, version change or rebuild is required by the
tested recovery compatibility result.

## Separate UI backlog: obsolete polling alert

Source review of [RxPanel](../../ui/src/xlog/RxPanel.tsx) confirms that a status-poll
catch sets the local `error` alert, while subsequent successful `api.status()`
acceptance updates the view/needs leases without clearing that error. A transient
poll failure can therefore leave an obsolete alert until an action or connection
effect clears it. This is a separate backlog defect, not an explanation for the
fresh owner uncertainty reason above. Add a failure-then-success recovery test
and clear only the appropriately superseded polling error when implementing it;
no runtime edit or new package is part of this checkpoint.

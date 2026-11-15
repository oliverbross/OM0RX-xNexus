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

## Owner correction installed; live repeat pending

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

The operator has been asked to pair the new owner session, confirm A/RX1 sharing
and start FT8 again. The new receiving outcome is pending: live 1.6.28 RX and
decoded-needs are **not yet qualified**. The 1.6.27 failure above remains a
failure. Repeat live receive/needs acceptance before promoting either capability;
the broader project, TX and updated platform/distribution gates remain incomplete.

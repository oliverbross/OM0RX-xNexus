# Live receive clock failure — 15 November 2026

The operator completed native pairing, confirmed slice A → DAX Audio RX 1,
enabled owner sharing and started FT8 receive in installed xNexus `.4`.
The unchanged application at `ae35fc6435f861e38029049299833afe57de4885` paired
with the real xLog workspace using protocol 1 and displayed the live DIGU
observation correctly. This qualifies that narrow live pairing/display path.

Receive then reported the owner's clock-validation error. Two observations
46 seconds apart showed zero decoded rows and no decoded-needs evidence.
The operator stopped RX sharing; it remains stopped. Successful pairing does
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

The owner implementation, deterministic tests, new xLog version, package and
installed acceptance are pending. No xNexus runtime change is proposed for this
incident. After owner qualification, repeat live receive and decoded-needs
acceptance before promoting either capability. The broader project, TX and
updated platform/distribution gates remain incomplete.

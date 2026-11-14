# Owner-mediated FT8/FT4 transmit design draft

**Not implemented; not enabled; not station-qualified.** This document proposes
the remaining M6 transmit boundary. The current companion remains receive/control
software. No endpoint, field name or numeric limit below is a released contract.
Broad project authorization does not authorize live RF. Transmit implementation
and supervised station acceptance are separate future work.

## Review provenance and existing source

Source review on 14 November 2026 used xNexus
`1b4ddbcbdcf763e791e2062e3092d650d65f0c15` and xLog
`72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5`. This draft incorporates the
owner review from the **xlog work** chat against that xLog source on the same
date. Its documentation branch starts at qualification `446489f49`.
Source references describe reusable pieces, not completed integration.

| Source | Existing behavior and reuse boundary |
|---|---|
| [Companion protocol](PROTOCOL.md), [receive adapter](../../src-tauri/src/xlog_decode.rs) | No companion TX API. The receive adapter owns neither Engine/StationCore nor hardware. RX clock provenance cannot grant transmit authority. |
| [xLog bridge capabilities](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/prototype/src-tauri/src/companion_bridge.rs#L172) | Existing observation/control/log APIs and Windows RX capability do not provide digital transmit. |
| [Mode implementations](../../crates/modes/src/mode.rs), [WSJT77 helpers](../../crates/modes/src/wsjt77/mod.rs), [golden tests](../../crates/modes/tests/wsjt77_golden.rs) | FT8/FT4 tone and waveform generation can be isolated from the station engine. `Mode::encode` is not strictly pure: it can queue `encode_later` memo work or call the vendored encoder. Use an explicit pure supported path, or a bounded documented fallback completed off the timing-critical path. |
| [Existing sequencer](../../crates/tempo-core/src/qso.rs), [native FLEX implementation](../../crates/tempo-audio/src/flex/mod.rs) | Sequencer logic is a reference for later exchanges. Do not instantiate Nexus's FLEX/DAX session or radio owner in companion mode. Start with one explicit message and one period, without automatic repeat or logging. |
| [Shared acquisition and intent](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/prototype/src-tauri/src/carrier_tune.rs#L12) | `OUTPUT_ACQUISITION`, `reserve_output`, and the revocable output intent already coordinate Tune/PTT/CW acquisition. Extend this authority to make the future prepared digital operation visible; do not introduce a parallel mutex that competing owners cannot see. Urgent release bypasses acquisition. |
| [Radio-slot ownership](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/prototype/src-tauri/src/radio_slots.rs#L284) | Claims validate the active route; same-radio acquisition can be idempotent. `release_tx(radio_id)` alone is insufficient for digital ownership: release must match an exact retained operation and must not clear its replacement. |
| [PTT session](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/prototype/src-tauri/src/ptt_session.rs#L15) | Target generations, separate acquisition/release locks, journals and uncertain release duties are useful foundations. A current GUI target obligation is insufficient: OFF must carry the exact retained operation token. Separate locks do not prove independent transport. |
| [Station ownership](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/Sources/XLogPrototypeSupport/LiveStationSession.swift#L19), [FLEX release](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/Sources/XLogPrototypeSupport/LiveFlexSession.swift#L304), [FLEX transmit observation](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/Sources/XCatIntegrationCore/FlexRadioClient.swift#L217) | Reuse xLog's commissioned/inhibited owner and observed transmit state. Observe and refuse external/Maestro TX; never release another owner's transmission. A slice's RX DAX channel is not a commissioned TX output mapping. |
| [Native voice playback](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/prototype/src-tauri/src/native_voice.rs#L544), [audio route/output](https://github.com/oliverbross/OM0RX-xLog/blob/72c4f00956b4eeaa3f4addb1ed8dfa5ee525c0d5/prototype/src-tauri/src/voice_audio.rs#L388) | Existing native voice primes silence, confirms PTT, then enables a following callback; it has route checks and a drain watchdog. This is not slot scheduling. Its WAV preparation clamps/resamples, so digital PCM needs a defined path without silent transformations. Windows/macOS audio support does not establish Linux transmit qualification. |

## Missing owner APIs and implementation decisions

xLog must remain the sole radio, PTT and audio-output owner. xNexus may generate
bounded modem content and request a single owner-scheduled operation. It must not
open CAT, SmartSDR, DAX or an audio device to transmit. Required additions are:

1. A locally commissioned digital TX route and explicit operator enable in xLog,
   with an optional capability absent when the platform/route is unsupported or
   unqualified. Pairing, RX availability and a selected radio are not consent.
2. An operation-token owner spanning preparation, acquisition, output and exact
   release, integrated with the existing shared acquisition/revocation authority.
   Account for CW, voice, Tune, GUI PTT and external/fldigi use; unresolved owner
   state inhibits acquisition. No second digital-only ownership mutex.
3. A bounded prepare/commit/status protocol with one-shot durable deduplication,
   plus a release-only channel and owner-local watchdog that do not depend on
   decoder, log, UI or ordinary query progress.
4. A calibrated period scheduler and final-PCM output path, using host UTC to
   monotonic conversion and CPAL presentation timing/frame offsets. Sleeping to
   a boundary and enabling the next callback is not sufficient.
5. End-to-end observed release and recovery semantics, including blocked worker
   stdin/driver transport and unknown release. Existing locks alone cannot meet
   that requirement.

## Proposed bounded protocol

Names such as `digital.tx` and `/xlog/v1/tx/...` are illustrative pending owner
agreement. Initial scope is FT8/FT4, one prepared period and at most one active
period per owner. No generic streaming queue, automatic next slot, late catch-up,
automatic retry of an uncertain start, or rearm after reconnect/restart.

Authenticate and validate capability before allocating a PCM body. Owner limits
must cover request bytes, decoded bytes, sample count/rate/channel format, text,
tones, finite numeric values, amplitude/peak, audio offset and preparation time.
Reject malformed, over-limit or refused messages before acquisition. A direct
generator at an owner-approved rate is preferable to a WAV conversion path.
Digest the **final PCM actually presented**, with a canonical format descriptor;
a digest proves byte identity, not that the bytes encode the asserted message.
Golden vectors and, if chosen, independent message validation provide separate
evidence. Do not silently normalize, clamp, resample or truncate after approval.

| Operation | Required semantics |
|---|---|
| Enable/lease | xLog-local explicit enable binds a short-lived acquisition lease to the complete route and commissioning generation. Return a separate exact-operation release authority that cannot acquire TX. |
| Prepare | Accept one bounded immutable body, operation ID, mode/message/offset, final PCM descriptor/digest and requested owner-time slot. Resolve and retain the complete owner context; complete any encoding fallback before the scheduling deadline. Return prepared ID, accepted context/digest, expiry and owner timing observations. No PTT side effect. |
| Commit | Atomically consume the prepared operation once. Revalidate context, intent, external TX, clock and deadline under the existing shared acquisition authority. Journal the one-shot decision durably before a start side effect. Same ID and identical body returns recorded status only; changed body conflicts. |
| Status/receipt | Report preparation, armed, active, completed, cancelled, failed or uncertain as owner facts with exact operation/context. Audio drained or a command ACK is not proof RF stopped. A receipt must distinguish commanded release from observed RX. |
| Stop/release | Independently mute/flush the exact operation and initiate its retained release obligation, retire pending intent, and reject late commit/PTT callbacks. Release remains possible after ordinary acquisition lease expiry; pairing revocation itself starts owner release. A stale ticket cannot stop a replacement operation or another radio owner. |

The bound context must include server/pairing/workspace epochs, stable radio and
provider identity, selection/worker/connection generations, exact TX slice/VFO,
dial and actual TX frequency, radio mode and split, commissioned output device/
DAX TX mapping and generation, sample format/rate/channel, audio offset, drive
and PA policy, and explicit enable/commissioning generation. RX slice A/DAX N
must not be treated as this mapping. Any relevant change retires the pending
start; revalidation is required at prepare, commit, acquisition, first frame and
during active output. Counter exhaustion fails closed rather than wrapping.

One-shot identity must survive process failure. Persist consumed preparation and
uncertain release duties, but **never replay a transmit on restart**. A lost
reply is resolved by status, not by a fresh operation ID. Journal failure before
the side effect refuses transmission. A crash after a possible side effect
retains an unknown release obligation and blocks new starts until resolved.
Do not let radio-ID-only cleanup clear the next operation's claim.

## Waveform and timing contract

`Mode::gen_wave` supplies a mode-specific lead-in plus waveform, not necessarily
the entire FT8 15-second or FT4 7.5-second period. Define exactly one lead-in and
explicit trailing silence to a reviewed total period length, with sample zero
at the agreed period boundary. Reject overflow rather than truncate. Padding
here is intentional scheduled silence, not invented samples repairing transport
loss. Transfer a complete prepared period before arming; incomplete transport
must never key a transmitter.

Agree UTC-to-monotonic calibration bounds, presentation-clock quality, scheduling
lead time, maximum lateness, PTT lead/tail, sample rounding, output latency and
clock-discontinuity behavior. Bind the preparation to that calibration. Late or
uncertain timing cancels rather than moving to a later slot. The current RX
`utc_sync_accuracy_us = null` and unknown RF latency remain unknown; they cannot
be promoted into a TX timing guarantee.

The owner schedules the audio frame position using presentation timing, while
holding only the necessary operation authority; it must not hold a general
bridge/state lock while waiting for a boundary. An owner-local watchdog releases
on period end, lease/heartbeat expiry, route/device/clock failure, shutdown,
workspace or pairing changes and cancellation. UI polling is not a heartbeat.
Exact durations and their measured worst-case bounds remain to be agreed.

## Independent release and recovery

Stop must remain useful while ordinary HTTP, modem work, logging, worker stdin or
a driver call is stalled. Prove the whole release transport, not only separate
mutexes. If the current worker path cannot provide this, an owner-approved
independent release transport and radio/device watchdog are prerequisites for
enabling the capability. Do not create an uncoordinated second radio session.

Revocation must retire preparation before late callbacks can acquire output.
Use exact retained operation tokens at every ownership layer, including GUI PTT
OFF and radio-slot cleanup. Release retries, if allowed, target only that retained
obligation; a retry must never key or retarget the current selected radio.
Unknown OFF remains journaled and visible, inhibits new acquisition, and is not
reported as success merely because the audio buffer was drained. External
Maestro or another owner's TX is neither adopted nor released by this operation.
Process death cannot guarantee physical release without a separately qualified
radio/device watchdog; that limitation must remain explicit to the operator.

## Offline implementation and test gates

These gates use generated signals, fake clocks, disposable journals and inert
owner/device doubles. They authorize no station output.

| Area | Required cases |
|---|---|
| Encoder/content | Actual FT8/FT4 golden tones and waveforms; refused/hashed/unsupported messages; pure-path and fallback bounds; finite amplitude; final-format digest; correct lead-in/trailing silence; no silent conversion or truncation. |
| Authentication/bounds | Authenticate before allocation; body/sample/rate/format limits; malformed and oversized payloads; absent capability; uncommissioned route; unsupported platform. |
| Clock/output | Period boundary/frame offset and sample rounding at approved rates; early/late arrival; UTC jumps; expired calibration; presentation clock unavailable; first/last callback; no output from incomplete preparation. |
| One-shot recovery | Duplicate identical commit; conflicting body; journal failure; crash before/after consumption and possible side effect; lost reply; restart status lookup without replay; same slot cannot transmit twice. |
| Ownership/fences | Voice/CW/Tune/PTT/fldigi competition; external Maestro TX; QSY/mode/split/drive/output mapping changes; both radio slots; worker/connection/pairing/workspace replacement; stale release cannot clear a replacement. |
| Stop/watchdog | Stop before prepare/commit, queued and active output; late PTT ON callback; expired/revoked acquisition lease; blocked decoder/log/HTTP/worker stdin/driver; independently measured release deadline; unknown OFF survives restart and blocks rearm. |
| UI/defaults | No TX in default companion mode or from a decode click/need score; explicit route/slot/body confirmation; Stop independent of exchange state; unknown timing/release visible; no automatic repeat or QSO logging. |

The first offline slice should implement the owner state machine, durable
one-shot semantics and independent Stop against inert transports before any
hardware backend is wired. Next isolate/golden-test the modem and presentation
scheduler, then connect the bounded authenticated protocol. Only after those
gates pass should a separately versioned candidate expose a disabled,
commissioning-only transmit feature for supervised qualification.

## Unresolved decisions and supervised acceptance

The owner and companion maintainers must agree exact wire schema/capability,
limits, enable/lease/heartbeat/watchdog durations, journal retention, token
propagation through all owners, independently functioning release transport,
supported pure messages/fallback behavior, canonical PCM format, trailing padding,
approved output rates, clock tolerances, presentation latency, PTT lead/tail,
drive/PA interlocks and per-platform qualification. No values in this draft are
an implicit default.

Supervised station acceptance then requires an explicit operator decision and a
controlled test setup: verify the actual DAX TX endpoint and TX slice, UTC and
presentation timing, observed PTT release under failure, passband/split and power
policy, audio level/ALC, and emitted waveform spectrum and slot alignment. Test
disconnect, external TX and Stop without adopting or clearing another owner.
Record exact application/firmware/device/route versions and accepted bounds.
Windows, macOS and Linux each need their own supported-backend evidence. Source
tests, synthetic RX decoding and a successful build do not qualify live RF.

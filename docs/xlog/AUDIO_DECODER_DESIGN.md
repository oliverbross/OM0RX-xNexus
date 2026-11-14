# xLog-owned audio and Nexus decoder integration

This is an implementation design based on the imported source, not a claim that
the current bridge transports PCM or can transmit. The connected desktop can
reuse the native Nexus DSP without opening a radio, sound card or second logbook.

The initial native adapter is `src-tauri/src/xlog_decode.rs`: FT8/FT4 only, exact
signed 16-bit 12 kHz mono frames, owner-provided radio/receiver/stream coordinates,
generation and a UTC uncertainty bound of at most 50 ms. It consumes the first
`ModeKind::frame_samples()` samples from a period's start; FT8 requires 180,000,
while FT4's decoder frame is shorter than its 90,000-sample 7.5-second slot.
It uses the non-a7 `SignalSource::decode` path with AP disabled until an explicit
owner-backed QSO context is implemented. It does not register an IPC command,
subscribe to audio or expose TX. Generated FT8/FT4 roundtrip tests are included;
their actual execution result belongs in the build acceptance record.

The current xLog contest-recorder callback has neither authoritative radio/source
mapping nor a calibrated first-sample UTC bound. Arbitrarily mixed recorder
samples and callback arrival time do not establish those facts. Consequently the
owner must not advertise `audio.rx` yet. The decoder remains dormant until those
two owner-side requirements and an explicit stream contract are implemented.

## Receive implementation

The narrowest seam is `modes::NativeSource::from_kind` in
`crates/modes/src/source.rs`. `SignalSource::decode_streaming` takes a
`DecodeRequest`, a full-pass flag and a `DecodeSink`; it returns normalized
`modes::Decode` records. `DecodeRequest::iwave` is signed 16-bit mono PCM at
12,000 Hz. `Mode::frame_samples` and `Mode::slot_secs` describe the required
period. Its request includes search limits, depth, own/other callsign, QSO
progress, receive/transmit offsets, monotonic frame time and AP/contest policy.
No device, local database or network connection is required by this interface.

Implement a dedicated connected decoder worker around this seam:

1. Subscribe to xLog's explicitly granted receive-audio capability. Negotiate
   exact PCM format, bounded block size, receiver identity and stream generation.
2. Validate every block before queueing: protocol version, session, radio and
   receiver IDs, generation, sequence/sample position, format, rate, channel
   count, payload length, UTC sample anchor and monotonic clock anchor.
3. Assemble complete mode-sized windows against the sample clock, bounded in
   memory. A gap, stale block, QSY, mode change, lost session or generation change
   drops the incomplete period and clears cross-period decoder state. Do not
   synthesize a complete period by silently zero-padding a broken live stream.
4. Run the decoder off the UI and transport threads. Preserve its existing
   synchronization around Fortran global state and reset FT8 a7/HARQ under the
   owning modem guard. Cancel stale decode results by generation/context.
5. Publish decoded text, SNR, frequency offset, timing and capture context to the
   connected UI. Ask xLog's batch needs evaluator to rank calls against the
   authoritative log and award context. Never rank against an empty local log.

The existing `tempo-audio::frames::RxRing` is a useful bounded buffer, but its
`frame()` zero-pads an incomplete window. The connected assembler therefore
needs its own explicit continuity/completeness gate before calling the decoder.
The HTTP bridge's control DTOs alone are not a PCM transport. A separately
authenticated local binary stream (or bounded binary HTTP blocks for an initial
receive-only implementation) should be versioned and advertised as a capability.

For a later full sequencing UI, `Engine::new`/`with_settings` initially constructs
an in-memory `StationCore` and a native source without starting device/feed
threads. Its split `build_decode_job` → `run_decode_job` → `apply_decode_result`
seam keeps decoding outside the engine lock. However, the Engine also owns QSO,
upload and automatic sequencing state. Wrapping it requires external-log and
external-station policies; exposing ordinary shell handlers would bypass xLog.
The direct `NativeSource` route gives useful receive/decode capability first.

## Transmit implementation

`modes::tx_mode`, `Mode::encode` and `Mode::gen_wave` can generate a slot-positioned
waveform without a device. `tempo-audio::AudioBackend` has 12 kHz mono capture/play
methods, but the ordinary `Transceiver` owns a concrete `Rig` and calls `ptt`
directly. Do not reuse that loop unchanged in connected mode.

Add a separate xLog transmission contract before enabling any connected TX:

- A short-lived station lease identifies the operator/session, radio, active
  receiver/context and allowed mode/frequency. xLog remains the sole arbiter of
  PTT, interlocks, band/mode policy and emergency stop.
- Submit a bounded waveform envelope with unique request ID, expected generation,
  sample format/count/hash, gain, intended UTC slot/start, maximum duration and
  expiry. Validate the complete envelope before accepting it; stale requests and
  retries after an uncertain result must never produce a second transmission.
- xLog acknowledges accepted/rejected, scheduled, started and completed/aborted
  states separately. Hardware-observed state decides whether keying ended.
- On loss of lease, disconnect, underrun, deadline, stop or process failure, xLog
  flushes pending audio and runs its unkey/watchdog path. Reconnect never resumes
  a queued over. xNexus's Stop requests are idempotent and remain available while
  any normal command is waiting.
- Log completed QSOs only through the existing durable xLog submission and
  reconciliation path; decoder/sequencer memory is not a second authoritative log.

## Acceptance order

Start with generated or licensed recorded PCM, including real FT8/FT4 message
round trips, and a fake xLog stream. Test split/merged blocks, all malformed
headers, dropped/duplicate/reordered blocks, clock changes, QSY mid-period,
mode changes, stale results, disconnect and bounded backpressure. Then test an
isolated xLog loopback audio fixture with no device. Receive-only station trials
come before TX. TX needs a dummy output/rig harness proving deadlines, lease
revocation, duplicate suppression, stop and watchdog behavior before any explicit
operator-supervised hardware acceptance. None of the compile or client tests
alone establishes on-air readiness.

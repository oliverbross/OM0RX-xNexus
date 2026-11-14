# xLog-owned audio and Nexus decoder integration

The receive implementation preserves separate processes and station ownership.
xLog captures the explicitly selected FLEX/DAX route. xNexus consumes bounded
authenticated PCM and calls the existing native Nexus FT8/FT4 decoder without
opening a radio, sound card or second logbook. Physical reception remains
unqualified; see [RX_DESKTOP.md](RX_DESKTOP.md) for operator behavior and evidence.

## Receive implementation

The Windows xLog owner uses receive-only WASAPI and preserves endpoint timestamp,
silence and discontinuity flags. An explicit mapping binds radio, client, slice A,
observed DAX channel, stable audio endpoint, native format and selected channel.
It never guesses a default endpoint or DAX channel. It exports one channel as
native-rate signed 16-bit mono PCM without mixing or resampling.

The versioned `audio.rx` capability exposes authenticated bounded JSON at
`/xlog/v1/audio?after=<last-consumed-sequence>`. Each response carries server,
workspace, freshness and capture/epoch/route identity. Limits include 4,096 frames
per chunk, 32 chunks per response and a two-second retained owner ring. Revocation,
QSY, mode/route/workspace changes, stale or transmit/unknown station state and
capture faults stop or invalidate the stream. Start and stop remain xLog actions.

The native SDK validates identity, sequence/frame continuity, format, rate,
selected channel, payload limits, timestamps and authority before decoding PCM.
Queued batches carry monotonic expiry and shared session/receiver validity.
Disconnect, replacement, revocation or receiver shutdown invalidates retained
batches; no automatic capture restart or mutation retry is performed.

`integrations/xlog-rx` resamples through the actual upstream 64-tap FIR and
assembles complete periods. Its center offset is 31.5 input samples; 12 kHz
passthrough has no resampler delay. FT8 requires 180,000 samples over 15 seconds.
FT4 consumes its 72,576-sample decoder frame within a complete 7.5-second slot.
The assembler waits for the period and required lookahead, rejects missing or
reordered audio and never fills a broken live period with synthetic silence.

`src-tauri/src/xlog_rx_worker.rs` separates polling/assembly from native decoding
with a one-period mailbox and process-wide worker lease. Backlog drops partial
state. Session/route/epoch/mode fences reject late results; the WebView receives
bounded status and at most 200 decoded rows, never PCM. Neither thread holds the
ordinary bridge-client or emergency rotator Stop mutex during polling/decoding.

`src-tauri/src/xlog_decode.rs` calls `modes::NativeSource` through the non-a7
`SignalSource::decode` path, AP disabled and depth 1. The existing decoder's
Fortran/global-state synchronization remains in force. No ordinary local Engine,
automatic sequencer, capture device or transmit service is created.

## Time and authority

Sample-zero timestamps derive from the Windows capture endpoint clock mapped
through QPC to the precise Windows wall clock. The reported calibration bound
describes that host mapping only; it must be at most 50 ms. Global UTC clock
synchronization accuracy and remote RF-to-endpoint latency remain unknown and
are explicit null values in this protocol revision. A small host calibration
bound is not evidence of accurate RF arrival time.

Every decoded row retains host timing provenance, sample alignment and FIR
center offset. Gaps, context changes and clock steps invalidate partial periods
and stale decode results. A complete generated fixture proves software behavior,
not physical clock quality or a DAX route's association with the intended radio.

Decoded-call batch needs evaluation remains future work. Current decoded text
is a receive observation; it does not classify a needed station, tune, transmit
or log automatically. The existing needed board uses authoritative xLog cluster
classification, and history reads the owner log. Never classify against an empty
companion logbook.

## Transmit design — not implemented

Native modes can generate waveforms, but companion mode exposes no TX path.
Before enabling one, implement an xLog-owned contract with:

- A short-lived station lease bound to session, radio, receiver, mode and frequency.
- A bounded waveform envelope with immutable request ID, generation, format,
  sample count/hash, gain, intended start, maximum duration and expiry.
- Separate accepted, scheduled, started and completed/aborted receipts, with
  hardware-observed release rather than acknowledgement treated as success.
- Owner interlocks, independent Stop, unkey/watchdog handling, duplicate
  suppression and cancellation on loss of authority, underrun or deadline.
- No queued transmission resuming after reconnect; QSO writes only through the
  existing durable xLog submission/receipt path.

Receive trials precede TX. A dummy output/rig harness must first prove deadlines,
lease revocation, duplicates, Stop and watchdog behavior. Physical/on-air TX
requires separate explicit operator-supervised acceptance.

## Recorded acceptance

The actual owner buffer and HTTP transport, real SDK, FIR/period assembler and
native decoder passed paced 48 kHz FT8 and FT4 generated-message tests without
relaxing response, sample-age or ring limits. A threaded worker test additionally
verified FT4 reception, Stop clearing, worker lease release and restart without
old results. Native adapter tests decoded both modes at 12, 24, 44.1, 48 and
96 kHz. Client and assembler tests cover malformed data, sequence gaps and
authority invalidation; UI checks cover receiving, Stop and pending-start races.

These are synthetic source tests. Candidate packaging, Tauri IPC/native GUI,
active DAX capture, real endpoint timing, overload shutdown timing and physical
station qualification remain separately recorded gates.

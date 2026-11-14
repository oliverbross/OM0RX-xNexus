# Receive-only desktop extension

The base receive extension was integrated into the source for `1.24.1-xlog.2`.
That candidate is not qualified for use; see the [current ledger](STATUS.md).
Later source adds xLog-owned classification to decoded rows. Installed
application and physical-station acceptance remain pending.

1. In xLog, explicitly select and confirm the FLEX slice A / DAX input route,
   choose its native format and channel, and start RX sharing in Normal Logging.
2. Connect xNexus with a pairing that grants `audio.rx`. Select FT8 or FT4 in
   **Receive decoder** independently of the radio's DIGU mode, then choose
   **Start receiving**.
3. Wait for a complete uninterrupted host-clock period. An incomplete period,
   transport gap, changed route, expired authority or decoder backlog is
   discarded. The app never pads missing audio with silence.
4. **Stop receiving** stops xNexus's consumer and decoder. Capture/sharing remains
   under xLog's controls. Stop before selecting another decoder. A still-running
   native decode may need to finish before a replacement worker can start.

The selected mono samples stay in native code. The WebView receives bounded
route/status metadata and at most 200 decoded rows; it has no audio-device,
capture, transmit or raw-PCM API. An unambiguous decoded sender requests xLog's
classification when `logbook.classify` is advertised. The displayed needed,
worked or Unknown status, score and reasons come from xLog and expire with
their original authority. A pending log change suppresses old classifications.
Decoding never automatically tunes, moves a rotator, transmits or creates a QSO.
See [the decoded-needs verification](DECODED_NEEDS_VERIFICATION.md).

Timing derives from the Windows capture endpoint's host clock. Every row retains
the host mapping/calibration uncertainty, sample alignment, resampler center
offset and explicit unknown UTC synchronization/RF latency values. These are not
qualified RF arrival timestamps. Physical FLEX/DAX association, decoder success
and clock quality still require live-station acceptance.

Native lifecycle fences independently reject late session, route and decoder
results. The poll/assembler and decoder threads use a one-period mailbox;
backlog clears the partial window. A process-wide lease prevents repeated starts
from accumulating native decoder threads. These threads never hold the ordinary
bridge-client or emergency rotator Stop mutex while polling or decoding.

Historical base-extension validation: full UI TypeScript and Vite builds passed; 40 distinct
companion UI tests passed, including four receive lifecycle cases. The isolated
synthetic browser probe passed 14 viewport/theme cases plus receive controls,
route and rows at 1366×768 and 1024×768/175%. Three focused worker tests compiled
the exact worker with the real SDK, assembler and decoder, proving current
authority acceptance, stale epoch rejection and revoke/Stop clearing. A paced
48 kHz generated FT4 fixture then passed through the real owner HTTP endpoint,
threaded `Worker::start` polling/assembly/mailbox and native decoder. Stop cleared
the route/rows, the worker lease released, and restart exposed no old result.
The three tests passed in 14.44 seconds. Full Windows GNU desktop all-targets
compilation subsequently passed on exact runtime source
`e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c` with Rust 1.93.1, locked dependencies
and one build job. The isolated check took 16m32s; its receipt verified source
identity and the original focused-test dependency hashes before and after.
Only the existing `capture_bounded` dead-code warning remained. Build log SHA-256:
`2491daa8dc8829ab529650b0b39ec90688a1898d68e0a86f4538ce4d89bbac1c`.
Tauri IPC, native GUI, packaged lifecycle and physical DAX acceptance remain
pending. Generated fixture reception does not qualify the physical station.

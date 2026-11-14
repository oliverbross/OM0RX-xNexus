# Receive-only desktop extension

This source lives on the separate RX development branch. It is not part of the
frozen `1.24.1-xlog.1` core package, and has no installed or physical-station
acceptance yet.

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
capture, transmit or raw-PCM API. Decoded text is a receive observation and never
automatically tunes, transmits, classifies a needed station or creates a QSO.

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

Development validation: full UI TypeScript and Vite builds passed; 40 distinct
companion UI tests passed, including four receive lifecycle cases. The isolated
synthetic browser probe passed 14 viewport/theme cases plus receive controls,
route and rows at 1366×768 and 1024×768/175%. Two focused worker tests compiled
the exact worker with the real SDK, assembler and decoder, proving current
authority acceptance, stale epoch rejection and revoke/Stop clearing. These do
not execute the full Tauri IPC or threaded `Worker::start` loop. Full desktop
compilation, native worker-loop acceptance and physical DAX reception remain
required before a new RX candidate is packaged.

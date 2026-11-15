# Owner PCM waterfall — native implementation and qualification

Base `75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`. Focused native validation
passed using generated PCM and synthetic local HTTP authority. No application
GUI, device capture or hardware was used. This is a native backend for the
original Nexus Waterfall, not a replacement UI or completed cockpit integration.

`RxAssembler::push_observed` preserves the existing `push` contract and exposes
borrowed 12 kHz float samples only after the whole native chunk succeeds. It uses
the existing anti-alias resampler once. Short FIR startup is not zero padded.
The observer only retains a bounded real-sample window and offers it to a single
dedicated FFT thread. No Engine, audio device, second owner HTTP stream, recording
or transmit path is created. Existing receive acquisition remains explicit.

The native spectrum producer retains 2048 real samples, at most one pending
window, and at most ten windows per second of sample time. The consumer uses
`tempo_core::spectrum::power_spectrum`: Hann 2048 at 12 kHz, 640 display bins over
0–5000 Hz, raw FFT spacing 5.859375 Hz, display bin width 7.8125 Hz, normalized
linear dBFS [-120,0]. It does not normalize every row or invent warming-up bins.
Only spectrum windows may be replaced; decoder periods and PCM are unaffected.

Every input batch lease contributing to the window, including FIR input history,
is retained. Return validity is the minimum of those original leases and a
500 ms local picture-age bound. Reading cannot renew either. A new batch cannot
extend old sample authority. An FFT result rechecks worker/session/workspace,
epoch, route and mode after computation and again on read. Stop/reset clears
pending/results; the FFT thread retains the existing ACTIVE worker lease until
it exits. Stop wakes it but never waits on unfinished computation. Finished
threads are joined; a retiring thread prevents overlapping replacement workers.

## Native command and original component adapter

`xlog_rx_spectrum({after})` is registered only in the xLog-mode command handler.
`after` is null or `{worker_instance,worker_epoch,sequence}`; duplicate suppression
applies only to that exact cursor context. No frame is `null`, not a zero spectrum.
The native command performs no HTTP or FFT. There is no new owner capability:
the existing authenticated `audio.rx` receive path supplies the samples. Existing
Tauri default capability describes local application handlers as requiring no
additional plugin capability; no permission expansion is introduced.

The frame's exact Rust source is `src-tauri/src/xlog_spectrum.rs`. It carries
the existing decode-row `context`, sequence, remaining_valid_ms, sample rates,
FFT window length, original device-frame/UTC origin, first resampled output
index, twice the FIR center in input frames, floor-microsecond window start
and exclusive end, original host clock uncertainty, and explicitly null UTC
sync/RF latency. The inner `spectrum` matches original camelCase Spectrum:
`{row,loHz:0,hiHz:5000,source:"audio"}`. Source position metadata describes the nominal rational sample schedule, not an
exact analog center: the existing FIR rounds fractional phase to a 256-phase
table, capped at phase 255. Host time is nominal endpoint time, never RF UTC.

Desktop services must validate the current full context, preserve a monotonic
deadline based on IPC request start plus remaining_valid_ms, and pass only each
new valid inner spectrum to the original component. Empty/no-new data must not
scroll. Clear its retained renderer history on worker/route/epoch changes,
including a change back to the same frequency. The unchanged original component
timestamps rows with `Date.now()`; that is display-arrival time, not capture UTC.
UI typing/adapter, reset behavior and actual cockpit validation remain separate.

## Focused validation and remaining gates

The final native run passed all 34 Worker tests, including the existing receive/
needs checks and new spectrum behavior: real RxAssembler output through Producer
and original FFT at 12 kHz, 44.1 kHz and 192 kHz with split FIR chunks; original
same-session lease expiry despite fresh newer input; expiry after computation;
full read-context fences; bounded latest-only mailbox; latched FFT with PCM and
Stop progress; actual HTTP/SDK Worker stop, running gap=true, new epoch and
worker restart. Old cursors use u64::MAX so sequence growth alone cannot satisfy
the context-reset assertions. The gap test uses generated silence from the
actual PCM path; it does not invent warming-up bins or claim a physical source.

Final evidence in the isolated worktree:
`target/waterfall-check/attempt-d93820a3a57545479983aa53c7d081ce/receipt.json`
SHA256 `1745BA985C321631F1B95BC0817E75AE04DB192D31C374DCF6B0EED5886A1B59`.
Tests log SHA256 `E2C60E99B867C6478C4EF34B5F3CC800F7892CD0B958AB5B3D20096C49F8C289`.
The final job exited0 naturally, root exited, final active processes0, cleanup
confirmed, peak853,217,280 bytes, no memory or abnormal-exit notifications.
Policy remained2 GiB/one job/BelowNormal/300 seconds, admission8 GiB physical and
6 GiB commit, floors2/2 GiB, natural descendant drain3 seconds.

The first attempt separately passed SDK audio6/6 and assembler9/9, then failed
native compilation because a test module needed an explicit path. Its exact
fresh SDK/assembler rlibs and source hashes were retained and verified in the
native continuations. The second run passed30 tests but its new fixture used a
synthetic token shorter than the SDK minimum; that setup panic poisoned the
shared serialization mutex, causing three additional failures. Only the test
marker/format changed for the final34/34 run. These failed receipts and source
copies remain intact under `target/waterfall-check`.

DSP dependencies came from the retained passed GNU Rust1.93.1 core inventory at
`43e305f1d65ec27f6f72255a14f2eb0775e9b349`. DSP source and root lock were unchanged
versus this base. Actual SDK/assembler and Worker sources were freshly compiled;
there was no substitute runtime or second Engine. Before/after source, tool,
rlib and helper hashes matched. Relevant final source SHA256:

| File | SHA256 |
|---|---|
| src-tauri/src/xlog_spectrum.rs | 930688F43E5D6DF26218BC8498EF5C271CAB9E247ABAE116C1EF1A0E7A6BD194 |
| src-tauri/src/xlog_spectrum/tests.rs | 6962150BEFFE573C511BB582FCBCEC43B1FE53B20592AFE1FFDE671A9DD9CB52 |
| src-tauri/src/xlog_rx_worker.rs | 5C0825B60C4CB3E10B37F67C6B17C2F35F815A4EB52543B9CD9A04573B094596 |
| src-tauri/src/xlog_rx_needs_transport_tests.rs | 07F48B1F8513A23BBB57403209909D914800ADE4FA34ED3B000674F8738FBBE8 |
| integrations/xlog/src/audio.rs | 6DA060AD10A6A2D66787F73FE2F9B622BDD54705BFA9371C4A92671DFBF1D4D2 |
| integrations/xlog-rx/src/lib.rs | 218D342B9278C99731662C90AAC6B0711D99AE4FF8C190B11F5FF85F62EF3E4E |

Remaining gates: full Tauri command compilation, exact Rust-produced envelope
fixture for the original UI adapter, integrated cockpit rendering/expiry/reset
and physical receive qualification. The focused wrapper compiles actual Worker
modules but does not compile xlog_mode's Tauri command registration. A serialized
frame is a snapshot, not a live lease: the UI must enforce request-start expiry
and current context, and clear retained history on invalidation. No usable
cockpit or physical receive qualification is inferred from this focused run.

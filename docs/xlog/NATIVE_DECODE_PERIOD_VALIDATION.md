# Native decode-period preservation

This slice preserves actual `modes::Decode` records at the existing receive
worker acceptance boundary. It does not instantiate or feed a Station, create a
transmit intent, add a callback/queue/thread/IPC, or enable any output backend.
The later sequence integration described in
[the adapter plan](TX_SEQUENCE_ADAPTER_PLAN.md) remains separate.

`Decoder::decode_native` runs the existing validation, reset, `NativeSource`,
AP-disabled/depth-one decode path once. `NativeDecodedPeriod` owns the resulting
vector, exact owner identity, decode context, host period start, independent
sample-counter time and `ClockProvenance`. It has no Serialize or Clone
implementation. Only the decoder's retained owner context is cloned; the returned
owner and native records are moved. No native metric is reconstructed from UI
text or presentation fields.

The compatibility `Decoder::decode` delegates once and consumes the result through
`into_presentation`. The existing `DecodedPeriod`/row/clock JSON is unchanged.
The worker calls `decode_native`, retains its existing cancellation, job-epoch
and audio-authority checks before/after decode, and converts to presentation only
inside the already accepted `Ok` branch. Raw owner identity and extra decoder
metrics are not sent to the UI. No decoder policy or lifetime is expanded.

## Focused actual-source qualification

On 15 November 2026, the uncommitted two-file delta over exact base
`37c0c0f1da88ad1ab86c7be3de91f0f359f579e0` passed all **44 tests**: eight decoder
tests (three new) and all 36 existing worker controls, with no failures, ignored
or filtered cases. Test execution took 40.71 seconds.

The new generated FT8/FT4 cases compare complete actual decoder records with an
independent native decode, including metrics absent from the presentation DTO.
They preserve full owner coordinates, the original owner string allocation,
context, sample-counter time, host period start and clock uncertainties/filter
offsets. They verify unchanged presentation fields, wrapper equivalence, duplicate
rejection/reset and a full owner change. Existing validation, generated native-rate
matrix, freshness/revocation, late-result, Stop, decoded-needs and waterfall
positive/negative controls also passed. The independent second DSP invocation is
only a test oracle; production still decodes each accepted frame once.

Evidence is retained in the isolated worktree under
`target/native-period-attempt-6f0a24af96b84c75ae0f88e180b2b0f5`.

| Source/evidence | SHA256 |
|---|---|
| Actual decoder source | `4851D0645BA1071838672EDDEB3E036007A1B9C0135D8E993AE1C60D03BACDB0` |
| Actual worker source | `577E510C4A226C8CB271FE19089200A382BD324609BA73B0B1F6CCD5F1D8DF5E` |
| Reviewed inputs | `979B8A86CAF037E8A2DD2AFF7ED65335C23618883CE531D1B87C4AFE5AA0A3A9` |
| Final receipt | `C30D85498BAACCF6CCCD088DDEC34F9E0B0C27D0A7B4DB9AB574072BD8356FB6` |
| Guard | `8DF26EEC052E900BCD8EB1607D6BE0162EAF65B7DC11C2110CE7DEC47CEB658E` |
| Exact checks | `ABF160FAA05D3A840C385A8E68CC063ECB052CA8985EF252614FD4F6A645D006` |
| Test log | `E2B2C93E993A5ADA65E0EEAF428831D062F44DD2CC1507128C6655FDF4C8966D` |
| Fresh test executable | `903B74F8C925F107739662FFA76BC79BBF2004BE096DDBD100105452A656D0CB` |

GNU Rust 1.93.1 directly compiled the current SDK and assembler as ordinary
libraries, then the actual decoder and worker modules as a focused test binary.
All 40 source, 50 tool and 1746 read-only dependency pins were unchanged before
and after. The retained core dependencies and entire `crates/`, `libtempo/` and
root lock contents matched qualified dependency source
`43e305f1d65ec27f6f72255a14f2eb0775e9b349`. There was no Cargo/CMake rebuild.

The reviewed guard used a 2 GiB aggregate cap, 300-second deadline, fresh
8/6 GiB physical/commit admission, 2/2 GiB runtime floors, BelowNormal priority,
sequential one-codegen-unit compilation and five-second natural drain. Root and
descendants exited naturally: process/effective exit 0, final active 0, no forced
cleanup or memory/abnormal-exit notifications, 18 ms drain. Peak job memory was
819322880 bytes; minimum available physical memory 20038950912 bytes and commit
18550968320 bytes.

This is actual-module source qualification with synthetic PCM and loopback owner
fixtures. The subsequent [combined native/Tauri normal-and-test metadata check](COMBINED_NATIVE_CHECK_2026-11-15.md)
passed on integrated source a0a804; it executed zero tests and remains distinct
from this 44-case runtime evidence. No application, GUI, device, waveform output, package, software
version change or RF action occurred. Host-clock provenance remains explicitly
different from qualified global UTC or remote RF timing.

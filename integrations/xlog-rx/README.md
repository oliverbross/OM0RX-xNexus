# Receive-only period assembly

This future-candidate component consumes already authenticated, bounded mono
PCM from xLog's Windows endpoint owner. It has no transport, station engine,
device, logbook, GUI or transmit interface. Hardware mode (for example `DIGU`)
is distinct from the operator's local FT8/FT4 decoder selection.

`RxAssembler::push` accepts an exact owner identity and at most 4096 native-rate
samples. Sequence, sample counter, format, clock or context discontinuity drops
the partial period and FIR history. The caller must also call `reset` on an
expired/stopped/unavailable response, explicit gap, authority loss or selection
change. Consumers must reject completed worker results from old generations.

The actual Nexus `CaptureResampler` supplies continuous anti-aliased 12 kHz PCM.
Its 31.5-input-sample center offset is accounted for (zero for passthrough), and
its 64-tap window reads only real samples. Mid-slot attachment waits for the next
complete slot. No partial window, lost packet, stream ending or filter tail is
zero-filled. FT8 collects 180000 samples; FT4 collects the leading 72576 samples
of its 7.5-second slot. Both wait for the whole slot before publication.

Timestamps describe the **host endpoint clock**, with a maximum 50 ms mapping
bound. The fixed sample-count anchor is checked against each owner's timestamp;
excess drift/clock steps discard the period and re-prime on subsequent input.
`utc_sync_accuracy_us` and `remote_rf_latency_us` remain `None`. These decodes
cannot establish UTC/RF timing qualification or authorize transmission.

The normal Cargo crate uses the existing `tempo-codec` dependency. For fast
assembly-only verification without building unused Opus/Tauri/device code,
compile the maintained harness which imports the **same assembler source and
same upstream FIR source**, not test substitutes:

```powershell
rustc --edition=2021 --test -C opt-level=1 integrations/xlog-rx/tests/standalone.rs -o target/xnexus-rx-pure/assembler-tests.exe
target/xnexus-rx-pure/assembler-tests.exe --test-threads=1
```

The desktop adapter checks these frame sizes against Nexus's `ModeKind` and
exercises real FT8/FT4 waveforms through the assembler and native decoder at
12/24/44.1/48/96 kHz. The modem's fixed samples-per-symbol waveform is generated
at 12 kHz, then independently interpolated into the synthetic endpoint stream;
changing the generator's sample-rate argument alone would change its baud rate.
Five focused adapter tests passed using the actual source and the pinned
dependencies from the previously passed core native build. The pure harness
passes 13 tests (six assembler cases and seven upstream FIR cases). Full future
desktop compilation, native IPC and physical endpoint acceptance remain separate
gates; these focused tests do not establish them.
Synthetic tests do not qualify an actual endpoint, radio mapping, UTC clock or
remote RF latency.

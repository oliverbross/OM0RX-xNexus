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
passes 14 tests (seven assembler cases and seven upstream FIR cases). Full future
desktop compilation, native IPC and physical endpoint acceptance remain separate
gates; these focused tests do not establish them.
Synthetic tests do not qualify an actual endpoint, radio mapping, UTC clock or
remote RF latency.

The maintained `scripts/xnexus/test-rx-owner-chain.ps1` also compiles the actual
owner `companion_bridge.rs` and `companion_rx_buffer.rs`, actual SDK and adapter
against hashed dependencies from a previously passed GNU native build. It
requires exact owner/SDK commits, checks those production files before compiling
and after execution, selects exactly two tests, and writes a receipt with source,
dependency and executable hashes. Unrelated owner worktree edits do not enter the
harness. This is a focused test executable, not a desktop application build.

Both paced 48 kHz cases passed using owner `f5c4048e805216edd3bc018a1f3c1bf23860e453`,
SDK `a23b01c477affd697e6a7d916bfab0db6dc78c7b`, and native dependencies qualified on
core `43e305f1d65ec27f6f72255a14f2eb0775e9b349`. Generated FT4 and FT8 messages
traversed 652 and 1088 genuine HTTP audio chunks respectively and decoded
`CQ K1ABC FN42`. Capture publication follows real wall time; owner retention,
client freshness and future-timestamp bounds remain enabled. Unknown UTC sync
accuracy and RF latency survive to the decoded result. This covers synthetic
capture through the production buffer, authenticated HTTP, client, assembler
and modem, while actual endpoint capture and desktop IPC remain separate gates.

Optional `-WorkerSource <file> -WorkerSha256 <exact hash> -WorkerOnly` compiles
the desktop worker source as well and selects its two authority tests plus a
paced FT4 owner HTTP lifecycle test. The
receipt preserves its source hash, worktree base and whether the worktree was
dirty, allowing a tested precommit file to be tied to the later identical commit.
These tests passed with a real loopback SDK response: current authority accepts
results; old epochs, reset and Stop reject them; revocation clears rows and route.
The lifecycle test executes actual `Worker::start` polling, assembly, bounded
mailbox and decoding threads, observes a decoded row with retained clock facts,
then verifies Stop clears rows and route, the exclusive worker lease releases,
and restart contains no old result. All three passed in 14.44 seconds against
worker source SHA256 `45A627F75521F17827FA0DE7B50D0C76BD86BCED8BF0769D76D069E52DFC56E3`
(desktop commit `531a6ec50`). Thread overload and a physical endpoint are not
qualified by this normal-load lifecycle case. The original two direct owner
chain cases remain separately recorded. Full Tauri IPC and installed application
acceptance remain pending.

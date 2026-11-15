# Paired implementation acceptance

This opt-in test crate compiles the real xNexus client and the real HTTP transport
from a separately checked-out xLog. It is outside both application workspaces and
is never packaged. The default provider is synthetic: passing it proves wire
compatibility, queue recovery and command admission, not SQLite or hardware.

On Windows, run `scripts/xnexus/test-paired.ps1 -XLogPath <checkout>`.
The command prints both source commits and reports dirty source explicitly.
Evidence must retain those coordinates; an uncommitted owner is not a release.

On another platform, set `XNEXUS_XLOG_SOURCE` to the absolute path of xLog's
`prototype/src-tauri/src/companion_bridge.rs`, then run
`cargo test --locked --manifest-path integrations/xlog-paired/Cargo.toml`.

The separate ignored `actual_swift_owner` case requires xLog's explicitly built
`scripts/companion-owner-tests` fixture. Pass its absolute executable path as
`-OwnerFixture` and optionally `-OwnerEvidenceDirectory`. Run in the prepared
Swift shell with the Swift runtime and matching SQLite DLL directory on PATH.
The runner records the fixture SHA-256 and preserves disposable evidence.
It creates a marked empty workspace, passes QSO operations through the actual
client, HTTP transport and Swift owner, restarts both sides after a simulated
lost acknowledgement, reconciles the durable receipt, then checks correction,
stale revision rejection, original-operation replay and history. The station
snapshot remains synthetic; no hardware commands or background station owners
are enabled. This does not substitute for native application acceptance.

The Windows `-Audio` option additionally compiles the owner's actual
`companion_rx_buffer.rs` and exercises selected-channel PCM through its real
HTTP endpoint and the xNexus audio client. It checks paged cursor continuity,
stop/restart with a new route/epoch, and revocation of queued audio. Owner buffer
tests also run. Only synthetic PCM enters the buffer; no WASAPI device is opened.
The runner records the buffer's hash in addition to the transport hash. This
feature requires the receive-owner source checkout and fails if it is absent.

For packaged desktop UI checks, build the separate `xnexus-ui-owner` binary with
the same `XNEXUS_XLOG_SOURCE` coordinate. The Windows launcher
`scripts/xnexus/start-ui-owner.ps1 -Executable <absolute exe> -ExpectedSha256 <hash>`
creates a private marked TEMP directory and verifies the exact live process path,
PID and argument before returning. It opens no application or station device.
Pairing credentials stay in its private `pairing.json`; do not print them.
The fixture provides a synthetic W1AW spot, in-memory contacts and synthetic
radio/rotor actions, recording commands in `events.jsonl`. By default it exports
no audio. For receive lifecycle checks, compile with `--features owner-audio`
and pass `-SyntheticAudio` to the launcher. This explicit marked fixture feeds
48 kHz mono silence through the actual owner buffer and HTTP endpoint at real
wall-clock pace. It opens no audio device and cannot produce a decoded message.
The route is deliberately labelled synthetic; DAX 2 is test data, not a station
default. Creating `STOP_AUDIO` stops sharing without stopping the owner; a
synthetic QSY also stops its stream. Neither action automatically restarts it.
If the producer misses a deadline by more than 250 ms, it stops instead of
publishing a stale catch-up burst. Use a fresh fixture for another capture.
Create its `STOP` file and verify normal process exit when finished; preserve the
fixture evidence. Launch the actual candidate separately with its explicit
private data root and verify that process before any GUI binding. This fixture
cannot establish actual xLog database or physical station acceptance.

The synthetic-audio extension passed a local Windows smoke check against owner
`5efffc1a23af68157e3c564eb2aa345256bea90a`: authenticated capability, DIGU route,
128 contiguous zero-PCM chunks, unknown UTC/RF bounds, sharing Stop, owner
survival after sharing Stop, and normal owner exit. Executable SHA-256 was
`4cab1b4e6e0d3a2ae9b2321a0299a40d872524087fd21197e174025f563b2d9b`.
Evidence remains in fixture `xnexus-ui-2b262104-bd6d-47ec-a4e2-d11ec625bbe5`.
An earlier fixture used an unsupported radio identifier and exited before
starting its bridge. Its evidence is preserved separately; the corrected fixture
uses the owner's allowed `radio1` value with explicitly synthetic endpoint and
receiver labels. No application GUI was bound during either sequence.


## Generated FT4 native needs acceptance (headless passed; GUI pending)

The separate `codex/ui-decoded-needs-fixture` acceptance source extends the
existing silence fixture without changing shipped application code. Build its
owner against the isolated xLog source **8bd486abd2e62aa079e8b4248126e1e186561452**:
`companion_bridge.rs`, `companion_rx_buffer.rs` and `companion_needs.rs` must all
match that commit. The classifier is explicitly synthetic, in-memory evidence;
this does not exercise Swift, a real database, a DAX endpoint or an RF clock.

Repeatable validation and later GUI plan (requires a released compiler/UI slot):

1. Preserve original silence executable/receipts. Use a new evidence directory,
   GNU1.93.1 and the passed core rlib receipt/inventory from
   `target/xnexus-native-tests/43e305f1d65ec27f6f72255a14f2eb0775e9b349`.
   Hash those artifacts before/after; do not rebuild or overwrite them.
2. Compile `src/generate_ui_ft4.rs` with recorded `tempo_core` and dependency
   search path. Generate a fresh absolute waveform path; record generator source,
   executable and output SHA256. It encodes `CQ K1ABC FN42` using the real modem
   and produces exactly 360000 mono48k s16le frames using the independently tested
   interpolation from `integrations/xlog-rx/tests/owner_chain.rs`.
3. Compile the acceptance owner with `owner-audio`, actual pinned owner modules
   and recorded serde/serde_json/base64/uuid artifacts. No tempo dependency is
   added to the owner or product. Run its callback wrap/origin regression.
   Run a headless SDK+actual RX Worker check against this executable: pair, start
   FT4, observe actual K1ABC decoded row then Needed with explicit synthetic reason,
   create private `STOP_NEEDS`, observe Unknown while PCM progresses, remove that
   marker and require new authenticated facts, Stop RX, then stop the fixture.
   Also retain one silence-mode compatibility and malformed waveform test.
4. Only after those checks pass, launch the hash-verified owner with
   `scripts/xnexus/start-ui-owner.ps1 -Executable <owner> -ExpectedSha256 <hash>
   -GeneratedFt4Waveform <wave> -WaveformSha256 <hash>`.
   This copies the waveform into a fresh restricted private root. Its repeating
   7.5-second UTC-aligned periods allow the UI to pair/start at any point.
5. Launch the frozen `.4` app with its explicit private-root coordinate, verify
   exact host/worker process coordinates before UI binding, then pair, choose
   FT4, Start RX and observe the real row plus owner badge. Record screenshot and
   fixture `events.jsonl` needs query, then Unknown/recovery and Stop. No native
   GUI launch is authorized by this preparation document alone.

All source/compile checks must precede GUI acceptance. Use one bounded 2GiB
BelowNormal Job for the focused compile/headless attempt, fresh 4GiB physical
and 6GiB available commit admission, 2GiB floors, no automatic widening/retry.
Generated wall-clock endpoint timing is synthetic and cannot qualify UTC sync
or unknown DAX/RF latency. `STOP_AUDIO` stops PCM; `STOP_NEEDS` disables facts;
`STOP` revokes/ends only this marked disposable owner.

### 2026-11-14 focused result

Actual source was compiled and exercised by `scripts/xnexus/test-ui-needs-owner.ps1`
inside the reviewed private 2GiB Job: callback wrap/origin regression, malformed
waveform rejection before readiness, silence compatibility, actual native FT4
row → authenticated synthetic Needed (9854ms) → STOP_NEEDS Unknown with continued
PCM (10148ms) → fresh Needed (10855ms) → RX Stop/empty → normal owner STOP exit.
The exact candidate product source remains frozen `ae35fc6435f861e38029049299833afe57de4885`;
only acceptance tools differ. No GUI, physical device, real logbook or transmit
path was exercised.

Evidence root (outside Git):
`C:/Projects/OM0RX-xLog/.build/xnexus-ui-needs-evidence/attempt-a3b38c5b647f49af85b60d4664757f86`.

| Evidence | SHA256 |
|---|---|
| compile-receipt.json | `5B1247F83344A61FE6D14CF00FFFDCC528C9CEFAC1A50CE4777A99840B2CBE10` |
| guard.json | `3111CA68F59EBF70E27EEB406EE91B324CCC79746D5AA32DCA0B9FBC9AADC73B` |
| headless.json | `12858D0292D5DB7DE5350646FDFAEA272496CCEF18820DBD685A8CD2978FD5FB` |
| xnexus-ui-owner.exe | `231E1C0108478712AC1C13D4A0FE0D7A9A09677150B82824381DA215E7D83467` |
| generated-ft4.s16le | `CBD61186133FB9B8CAB0F16F550E993588246D1F4241F4BC47DFDDFB6B2196A7` |

The guarded job peaked at 777986048 bytes; natural root exit and zero active
children were confirmed before cleanup, with no forced termination. Original
rlib/source hashes were checked before and after. All outcomes were within the
unchanged five-minute limit and 2GiB host physical/commit floors.

The earlier `attempt-89fefb9608e54adeb473edd7a91715b0` failed its positive-decode
phase because the synthetic producer exceeded its inherited 250ms callback
lateness guard. It also exited naturally with zero children. That failure is
retained; its scheduling cause was not established. Only diagnostic evidence was
added before the successful second explicitly authorized attempt. The deadline
was not widened, skipped samples were not filled, and no automatic retry was
introduced. This is successful focused evidence, not a reliability guarantee
under arbitrary desktop scheduling or a substitute for native GUI acceptance.

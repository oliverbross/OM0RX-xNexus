# Original FT8/FT4 native coordinator

Implementation slice based on `f8be70d5122cec63b99ef9cd2511043dbf5c072a`.
The new native modules are not registered in the application yet. They do not
open a device, call xLog, issue PTT, schedule playback, or write a contact.

`src-tauri/src/xlog_ft_runtime.rs` owns exactly one original
`tempo_core::qso::Station`. CQ, Work with decoded context/skip-Tx1, auto-sequence,
parity, resend, selected-message override, free text, Enable and Stop are explicit
intents. Original Station supplies message progression, reports, retransmission
budgets and closing rules. The coordinator adds bounded pending intent and owner
correlation, not another exchange state machine.

## Native integration contract

1. Construct `Context` only from the current paired connection, saved identity,
   accepted receive context and independently commissioned xLog TX context/clock.
   RX mapping and the local call/grid do not themselves authorize transmission.
   `new` starts disarmed. Authenticate fresh owner leases before `refresh`.
2. Apply original UI intents to this coordinator. A current selected decode must
   be checked by the caller before supplying Work's message/SNR context. Options
   do not arm a stopped station. Mode/owner/identity/route/clock changes retire the
   old coordinator; retain its release handle until exact release is established.
3. Borrow each accepted `NativeDecodedPeriod` at the existing worker acceptance
   boundary, before presentation conversion. `observe` compares exact owner and
   decode context and consumes a period once. Busy periods are discarded; there
   is no decode catch-up queue. Receive clock metadata never grants TX authority.
4. `plan` accepts the current owner slot observation and freezes the original
   Station/message. Use `Plan::prepare` on a bounded preparation worker outside
   connection, decoder, coordinator and Stop locks. A stale result cannot bind.
   Same-slot high-water survives explicit QSO resets within this coordinator;
   restart durability remains the owner's journal responsibility.
5. Convert source PCM to the commissioned final format and obtain the owner's
   exact body binding separately. `bind` retains operation ID, whole-body digest
   and the complete local intent key. It is **not** a substitute for validating
   the owner's body against the final PCM. This trusted adapter is not yet wired.
6. Implement `OwnerEvents` only in the authenticated native SDK adapter. It must
   verify operation/body/context/clock/event identities and distinguish actual
   start, definite no-start, observed release and uncertain outcomes. There is no
   production fake event or serde/IPC boolean that constitutes such proof.
   Pending/prepare/commit ACK never calls `Station::after_tx`. Actual start does
   so once; its original timestamp must lie inside the planned start window.
7. Retain the independent `StopHandle` above worker/query locks. Its short cell
   revokes admission and returns the exact operation without waiting for encoding,
   DSP or owner queries. The SDK then sends independent exact-operation Stop.
   Unknown outcomes retain the obligation; only verified release/no-start clears
   it. Do not release a replacement operation or infer release from an ACK.

`View` is a native state view for mapping into the existing operating controls;
it is not an owner receipt. Tune is a separate xLog carrier-owner operation, not
an FT message or a locally generated output command. Completed contact publication
must use xLog's durable log boundary, independently of sequence progression.

## Encoding boundary

`xlog_ft_encode.rs` reuses the existing WSJT77 refusal/packing/tone functions and
FT8/FT4 waveform generators. Standard preparation is compared against the existing
`xlog-tx-prepare` source corpus path. That crate and its version-1 format remain
unchanged. The new native source buffer is full-period 12 kHz mono f32; it is not
a declaration of the DAX output format.

Free text must fit its actual 13-character alphabet. The upstream packer's silent
free-text truncation is refused. No new packer or implicit case normalization is
introduced. Where the pure tones depend on upstream memo state, the fallback is
the actual upstream `Mode::encode`, outside the timing-critical path. One global
preparation admission and captured deadline prevent multiple blocked FFI jobs or
late accepted results. In-process FFI cannot safely be killed: a timed-out job
must retain its preparation admission until it returns, while Stop remains
independent. This is not a hard execution-time guarantee for the encoder.

## Qualification

Final source passed 29 focused tests: 21 coordinator/encoder tests and eight
existing native decoder tests. These include actual generated FT8/FT4 periods,
original CQ/Work progression, duplicate/late owner-event rejection, independent
Stop, exact standard-preparer PCM correspondence, bounded free text/compound
forms, and actual upstream encoder fallback positive controls. Unresolved literal
hash placeholders are refused rather than silently interpreted as message proof.

Evidence (private task storage):
`owner-ft-runtime/target/runtime-attempt-0d64ae47c0d542958484fbe6d4f222ac`.
Receipt SHA-256 `D2182E5EA63242345AF1FB46EF99D820305D8F79AE178DF89CC61A97156EA7B5`;
test log `BFFB5AD9B2D34B6E716C740CF5219A03111D275C6E1106EA5D7156F6ED2E69B0`.
GNU 1.93.1 direct compilation used fresh native/preparer/assembler artifacts and
1,746 immutable dependency pins from source `43e305f1d65ec27f6f72255a14f2eb0775e9b349`;
its core/library/lock inputs match this base. Before/after pins were unchanged.
The 2 GiB, 300-second BelowNormal job exited naturally with root/final process
counts zero, no resource flags or forced cleanup, and peak 637,956,096 bytes.
The receipt's older prose label says runtime20; its actual checks and named test
log correctly record 21 runtime plus eight decoder cases. The earlier 28-case
pass is retained separately and is not substituted for this final-source run.

No full Tauri build, Cargo/Clippy, SDK owner transport, live actual-start event,
commissioned output, GUI, hardware or RF acceptance is claimed by this slice.
The modules are not yet wired into the application.

# FT8/FT4 sequence adapter plan

**Source review only: not implemented, enabled, packaged or RF-qualified.** TX
remains required M6 software work. This plan refines the
[owner-mediated TX design](TX_DESIGN_DRAFT.md) and [project plan](PROJECT_PLAN.md);
it does not reserve a candidate version or authorize a station transmission.

Reviewed on 15 November 2026 against xNexus
`cff6e74d7d99ced6a5b848446d84259767a19fef` and xLog
`70599ecf373801b3cbb5221290db8b6d965e5fff`. The referenced xLog files matched that
commit. Existing offline models are distinguished from missing live integration.

## Existing reusable seams

| Source/type | Reuse and constraint |
|---|---|
| [tempo_core::qso::Station](../../crates/tempo-core/src/qso.rs) | `monitoring`, `calling_cq`, `start`, `observe`, `outgoing_rv`, `override_next`, `resend`, `after_tx` and `State::nqso_progress` provide the exchange state machine without Engine, hardware or log construction. `observe` consumes message/SNR from `modes::Decode`; monitoring does not automatically answer CQ. Use `outgoing_rv` to retain call budgets; FT8/FT4 do not use its IR-HARQ redundancy value. |
| [Msg](../../crates/tempo-core/src/message.rs), [modes::Decode](../../crates/modes/src/decode.rs) | Preserve the original parser, directed exchange and on-air message rules. A UI row is not a new decoder event. Hash-dependent message forms require an explicit supported encoding policy. |
| [Engine::TxPlan/TxWaveform](../../crates/tempo-app/src/engine.rs) | Existing `plan_tx` → waveform build → `commit_tx` demonstrates context fencing, but cannot be delegated unchanged. Planning already calls `Station::after_tx`, changes counts and emits other state. Refused commit only reverses selected call-change cases. `poll_tx` also relies on its caller supplying a live slot. |
| [SlotClock](../../crates/tempo-core/src/timing.rs), [slot functions](../../crates/tempo-audio/src/slot.rs), [audio service](../../crates/tempo-audio/src/service.rs) | Explicit-time slot arithmetic is reusable. `wsjtx_tx_window_ms`, `wsjtx_may_key_at`, `wsjtx_over_end` and `boundary_keyed` document upstream timing and same-slot suppression. The service owns Rig/audio; do not instantiate it in the companion. Arithmetic is not a calibrated output scheduler. |
| [FT8/FT4 Mode](../../crates/modes/src/mode.rs), [wsjt77](../../crates/modes/src/wsjt77/mod.rs) | `ft8_refusal`/`ft4_refusal`, tone generation and waveform functions are available separately. `Mode::encode` can queue `encode_later` memo work or use the vendored encoder, despite a broader pure-Rust comment on `TxWaveform::build`. No timing-critical path may assume that fallback is lock-free. |
| [Decoder](../../src-tauri/src/xlog_decode.rs), [Worker](../../src-tauri/src/xlog_rx_worker.rs) | Current complete-period decode is owner/context fenced, AP disabled and depth 1. The worker verifies its epoch before/after DSP, then projects bounded display rows. `Worker::stop` stops receive processing; it is not a transmit release. |
| [OriginalHost](../../ui/src/xlog/OriginalHost.tsx), [OperateServices](../../ui/src/components/operateServices.tsx) | Original callbacks and `transmit`/`stopTransmit` action classes exist. The companion currently denies transmit callbacks. Future handlers must express sequence intent and owner operations without falling through to native station APIs. |

xLog already contains offline
[prepared_period::ValidatedPeriod](https://github.com/oliverbross/OM0RX-xLog/blob/70599ecf373801b3cbb5221290db8b6d965e5fff/prototype/src-tauri/src/prepared_period.rs),
[output_authority::Authority/Reservation](https://github.com/oliverbross/OM0RX-xLog/blob/70599ecf373801b3cbb5221290db8b6d965e5fff/prototype/src-tauri/src/output_authority.rs)
and [digital_tx_owner::Owner, Permit, ReleaseHandle, Journal and Transport](https://github.com/oliverbross/OM0RX-xLog/blob/70599ecf373801b3cbb5221290db8b6d965e5fff/prototype/src-tauri/src/digital_tx_owner.rs).
These provide immutable byte validation, shared reservation, durable one-shot
decisions and exact-operation release models. They have no live companion TX
endpoint, commissioned context builder, audio/PTT backend or physical release
qualification. Reuse them rather than introduce a competing ownership mutex.

## Adapter contract to agree before implementation

Keep one bounded sequence instance per owner identity, QSO epoch and FT mode.
Inputs are explicit operator intent, fresh complete decode periods and correlated
owner operation facts. Output is a proposed immutable message/period, never direct
CAT, DAX, audio or log I/O. Bound retained events, transcripts and pending work.
Mode, identity, route or clock retirement must invalidate pending starts; no
rearm, catch-up transmission or uncertain-start retry after reconnect.

Consume each admitted period exactly once before the worker's retained UI-row
projection. Prefer preserving the actual `modes::Decode` internally: current
`DecodeRow` omits sync, AP, quality and raw metadata. The sequencer presently needs
only message/SNR, but invented values must not masquerade as decoder evidence.
Selected-QSO AP context is absent today. Adding mycall/dxcall/nqso_progress needs
the same owner/QSO/period fence and reset discipline; another stream's memo or
prior-cycle a7 state must not leak into it.

`Station::after_tx` sets `keyed_on_air`/`keyed_as`, increments the step count and
consumes one-shot closing messages. Its future trigger must be an **agreed owner
actual-start event**, correlated once to exact operation, message, slot and QSO
epoch. Prepare success or commit ACK is insufficient. The definition of that
event, partial/aborted-output counting and unknown outcomes remain owner-contract
decisions. A late event cannot advance a replacement QSO. Completion is not a
direct log write: logging must use xLog's existing durable publication boundary.

Keep original CQ/call/message/enable/Stop interactions, but separate local sequence
intent from explicit owner enable, prepare and atomic commit. Stop must retire
pending intent immediately and use the exact retained release handle independently
of decoder, ordinary queries, waveform preparation or acquisition locks. Preserve
unknown release duties; never release another owner's or replacement operation.

## Waveform and clock decisions still open

The retained 12 kHz mono f32le full-period corpus is an **offline interchange
fixture**, not an agreed DAX TX format. RX slice/channel mapping grants no output
authority. Commission TX slice/VFO, output device/channel, format/rate, drive/PA
policy and generations separately in xLog.

FT8 `Mode::gen_wave` includes a 0.5-second lead-in; FT4 includes 0.3 seconds.
These are not necessarily full 15/7.5-second buffers. `slot_over_samples_12k`
separates lead and signal; define exact trailing silence, finite amplitude limits,
sample count and final byte digest before commit. No silent resampling, clamping
or truncation after approval. A digest proves bytes, not the claimed message.
The offline owner validator currently admits `standard_type1`; other original
Nexus message forms need explicit coverage or refusal, not an assumed fallback.

For a pure supported encoder path, reject `None` from the tone helpers and FT8's
all-zero refusal. Any permitted vendored/memo-dependent fallback must finish under
a bounded preparation policy outside the critical schedule. Golden vectors must
cover the chosen path. Keep encode/decode memo concurrency explicit.

Owner scheduling needs calibrated UTC-to-monotonic and actual output-presentation
timing, not sleep followed by enabling the next callback. Upstream reference
windows are 13.64 seconds for FT8 and 6.04 seconds for FT4, with a 200 ms PTT-off
delay; its late-start code can send only remaining symbols. Whether the owner
contract refuses all late starts or supports that behavior must be decided and
tested. Full-period buffering must not accidentally keep PTT asserted throughout
trailing silence. RX host-clock uncertainty is not transmit timing certification.

## M6 work breakdown and exit gates

| Work | Software exit gate, before any RF claim |
|---|---|
| Sequence event adapter | Offline two-station FT8/FT4 exchanges exercise actual Station/Msg, once-only period consumption and actual-start correlation. Duplicate/late owner events, A→B→A context changes, refusal and uncertain start cannot burn or replay a step. Bounded retention and original call-budget behavior are checked. |
| Immutable period preparation | Agreed supported message forms, golden tones/waves, refusal controls, complete-period geometry, amplitude and final-byte digest validate through the actual owner codec. No device or hardware construction. |
| Owner protocol and output authority | Authenticate before allocation; fixed byte/time bounds, complete route/clock/policy binding, atomic commit deduplication and body conflict. Recovery never replays TX; journal failure retains uncertainty. Stop before/during commit and stale release cannot affect another operation. |
| Live owner backend, still without RF qualification | Implement commissioned format, presentation-clock scheduling, first-frame/context checks, owner-local watchdog and independent mute/release. Test blocked ordinary/worker transport, expiry, clock steps, underrun and disconnect using a qualified inert backend; command ACK is not observed release. |
| Original UI and logging | Actual original controls map to the adapter; capability absence fails closed without native fallback. Explicit enable and global Stop remain distinct. QSO/receipt state and xLog publication are tested without inventing a local completed contact. Integrated frontend/native gates and a uniquely versioned private candidate follow. |

Supervised station acceptance is a **separate subsequent gate**: agree the actual
TX route/format and timing tolerances, verify release/watchdog behavior and output
under controlled station conditions, then test real exchanges and log publication.
Windows, macOS and Linux qualification are independent. Offline success, a built
candidate or a working receive stream does not enable transmission or complete M6.

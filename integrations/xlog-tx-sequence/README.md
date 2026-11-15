# Offline sequence adapter

**Offline module tests passed; not wired into any desktop or owner endpoint.**
This isolated crate uses actual `tempo_core::qso::Station`, `Msg`, `modes::Decode`
and the existing `xnexus-tx-prepare` producer. It does not construct Engine,
devices, network clients, a clock, a scheduler, a journal or a log writer.

`FakeActualStart` and `acknowledge_fake_release` are explicit synthetic test
inputs. Neither means Starting, Permit, ACK, physical keying or observed release.
The adapter never turns those intermediate acknowledgements into `after_tx`.
No output backend or live event contract is implied.

## State boundaries

- A new adapter is disarmed. Operator intent creates a private Station and checked
  context/QSO epochs. Station is never returned mutably.
- Each bounded, context-matching complete batch has a period high-water mark and
  local duplicate fingerprint. While an immutable intent is pending, even a Busy
  batch is **dropped and remembered**. It cannot be replayed after start. Nothing
  queues or invents missed receive periods.
- A prepared slot freezes Station and message. Only one exact correlated fake
  start applies `after_tx` to that frozen state. Refusal, unknown, expiry, Stop
  and context retirement halt until explicit operator intent.
- The exact pending or last-started operation remains retained for release.
  Replacement and another preparation are refused until an explicit matching
  fake release acknowledgement. A late old release cannot retire a replacement.
- Slot end high-water survives every QSO/context reset in this adapter, including
  FT8/FT4 changes. There is no process-restart durability claim; that belongs to
  the actual owner's journal. All epoch counters are checked, never wrapped.

Use `outgoing_rv`, preserving actual CQ/directed budgets. AwaitReport remains
deliberately uncapped by Station; a separate 4096-event offline-session bound
halts this prototype. RR73 and final 73 are one-shot; RRR remains pending under
upstream policy. This is not a live watchdog implementation.

## Opaque owner fixture boundary

`OfflineOperation::from_offline_fixture` retains the **entire bounded owner Body
bytes, manifest bytes and exact prepared PCM separately**. It checks sizes and
PCM equality, not Body schema, authorization, route validity or manifest truth.
The explicit offline coordinates are correlation only, not a new production DTO
or digest schema. Fake-start equality includes all bytes and coordinates.

The actual owner's `digital_tx_owner::Body` and `prepared_period::validate` must
cross-check a generated fixture in a separately pinned harness before any
qualification. Unit-test stand-ins are explicitly opaque and are not valid-owner
evidence. The existing export corpus's body digest is also fixture-only; no new
digest format is introduced here.

Preparation retains the existing 12000 Hz mono f32le full-period format and
strict standard-type1/refusal policy. It does not call `Mode::encode`, add AP,
alter memo behavior, resample, clamp, or authorize DAX TX. Other original message
forms remain explicitly unsupported by that producer.

## Bounds and pending qualification

One pending prepared period (at most 720000 PCM bytes), one immutable operation
with a separate PCM copy, 40000 Body bytes and 16384 manifest bytes are retained.
Prepared-intent clones share their period allocation. Caller-retained objects
are caller-owned; the adapter has no event/history queue. Batches admit at most
256 rows of 128-byte text, with finite measurements and no AP/HARQ/raw payload.
Station transcript is drained after bounded mutations into 32 diagnostics of at
most 1024 bytes each. Receive rows retain their real measurements; tests label
synthetic values explicitly. Current native display rows omit metadata, so no
native decoder subscription is implemented in this slice.

Source tests cover two-party Station exchanges for both modes, budgets/closing,
duplicate and dropped periods, exact fake-start identity, release retention,
retirement, refusal, same-slot overlap, counter exhaustion and size bounds.
The 22 adapter tests and six unchanged preparation golden tests passed under a
2 GiB / 300 second GNU Rust 1.93.1 guard. Source/dependency pins and natural
process retirement passed; see [validation evidence](VALIDATION.md) for hashes,
scope and the separately checked isolated lock. Actual-owner fixture cross-check
remains pending. No application, UI, live radio or complete M6 claim follows.

# RX recovery compatibility acceptance

This acceptance-only extension keeps all production xNexus SDK, assembler,
decoder and native Worker modules identical to candidate `ae35fc6435f861e38029049299833afe57de4885`
(`1.24.1-xlog.4`). It builds the separately pinned xLog Buffer/HTTP provider;
it does not open a device or real logbook, launch a GUI, or transmit.

`audio_owner.rs` tests declared packet loss and a contiguous packet whose raw
QPC/UTC residual exceeds 50 ms. Five subscriber cases cover current clients,
lagging clients that see the waiting reply, lagging clients that miss it,
caught-up clients that miss it, and clients joining after recovery. Assertions
cover no authority/PCM while waiting, monotonic global cursor, unchanged raw
frame/time/sample data, unchanged physical capture/route, new epoch, explicit
gap when unread chunks were flushed, and subsequent unflagged-gap rejection.
The raw timestamp has a six-tick remainder to exercise wire microsecond flooring.

The paced generated-FT4 fixture uses an explicit synthetic 100 ms delivery
latency. Its producer can consume one `GAP_REQUEST`, then one `TIMING_REQUEST`.
The latter advances both synthetic endpoint clocks by 60 ms, discards the
overflow packet through the actual owner API, and keeps that clock offset for
subsequent packets. Nothing is sent with an invented replacement timestamp.
Atomic `*_ACK.json` files record old/new descriptors and discarded positions.
No automatic retry or producer deadline relaxation is allowed.

The actual `.4` Worker must decode `CQ K1ABC FN42`, receive synthetic owner needs,
clear rows/facts after each recovery, then decode a complete new-epoch period
and obtain newly generated needs. Explicit Stop empties the view; the fixture
exits normally. This is a synthetic classification test, not Swift logbook or
physical DAX qualification. Separate unchanged `.4` Worker tests exercise the
epoch predicates and delayed-reply rejection; the headless chain does not claim
to latch a decoder mid-call.

Run `scripts/xnexus/test-rx-recovery.ps1` only inside the reviewed private Windows
Job guard, with an exact committed `-OwnerCommit`, `-OwnerRoot`, a fresh evidence
directory and the retained dependency receipt. The script refuses changed `.4`
production modules, owner modules/CSV fixtures, or dependency artifacts, and
records source/output hashes before and after validation. Preserve any failed
attempt. A successful compile is not a packaged application acceptance result.

## Validated result — 15 November 2026

One guarded run passed against owner commit
`9f75def1d41ec695165eca2cc016f2226b687651`. Both five-subscriber HTTP tests passed;
all 19 unchanged native Worker tests passed; the callback/waveform and headless
checks passed. Headless elapsed milestones were:

| Event | Elapsed milliseconds |
| --- | ---: |
| Initial decoded Needed | 10054 |
| Needs unavailable while PCM continues | 10147 |
| Fresh Needed | 11054 |
| Declared gap cleared rows and needs | 11443 |
| New-epoch complete period, fresh Needed | 25126 |
| Timing-overflow discard cleared rows and needs | 25486 |
| Next new-epoch complete period, fresh Needed | 40018 |

Stop cleared the final view and all disposable owners exited normally. The
2 GiB, one-job, BelowNormal guard peaked at 850,972,672 bytes. Root exited with
code 0 and zero active descendants, confirmed by the three-second natural-drain
check; no forced termination was requested. Total guarded run was 147.5 seconds.

Private evidence is retained at
`C:\Projects\OM0RX-xLog\.build\xnexus-rx-recovery-evidence\attempt-965df9caba324250ba0a540f77cff69e`.
Pairing files and private fixture directories are deliberately not committed.

| Receipt | SHA-256 |
| --- | --- |
| `guard.json` | `E9C2D261B13F7BA8B9C0055AC896ECD993E2C84BEAF1E59C085CE20F993F219E` |
| `compile-receipt.json` | `664C91915EE5EAEA60F60018EDFF1CA0B7430D0239C3364172DBEF03CBEC94DC` |
| `headless.json` | `9B24BD7FD850523254B029E0C813041EFB8CC70F9F48A64B84227BF33FB41635` |

The fixture base was the already-tested acceptance dependency `147ef0595`;
new test sources were uncommitted during validation and are bound by the
compile receipt's complete source inventory. They were unchanged through the
run and publication. An earlier mistyped source argument was rejected before
admission or child creation; it was corrected from `git rev-parse HEAD` before
this sole compile/test attempt. Full application compilation, packaging, GUI
and physical RF/DAX acceptance were not part of this test-only run.

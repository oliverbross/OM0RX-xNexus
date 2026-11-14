# Execution ledger — 14 November 2026

The full plan is being executed with separate desktop, xLog-owner and release
worktrees. This ledger distinguishes committed source from release acceptance.

| Milestone | Current evidence | Remaining acceptance |
|---|---|---|
| M0 | Complete: full upstream history, private origin, fetch-only upstream, nine issues/milestones | Maintain patch inventory on updates |
| M1 | Native client at `76a24d508`: 39 tests, Clippy and formatting pass. Real xLog bridge source implemented in isolated owner branch. Actual client/bridge TCP test passes with synthetic owners. | Owner branch compilation and canonical integration after xLog 1.6.21 acceptance; paired application test |
| M2 | Desktop `62f102f60` integrated: separate entry, native pairing, restricted invoke handler, no normal station startup. Queue UI `7049ffc19` adds four tests, bringing focused UI total to 12. Browser probe 14 viewport/theme cases reported passed. | Native build, final queue UI integration and isolated installed-app acceptance |
| M3 | xLog classifications/scores/reasons and optional country/entity/coordinates carried through native contract; unknown remains unknown | Owner projection regression tests, edited/imported/confirmed history correctness, complete paired views |
| M4 | Explicit tune/point/Stop client and owner adapters; identity/revision admission; independent Stop transport | Owner generation/concurrency tests and observed fake-hardware/application acceptance |
| M5 | Durable outbox `2f62d569b`; synced attempt before send, receipt reconciliation after crash, explicit same-ID retry, workspace fence | Owner durable transaction tests and paired create/update/conflict acceptance |
| M6 | Decoder-only source seam identified; no hardware or empty local log engine needed | Authoritative xLog PCM mapping and timestamp calibration; RX validation; separate TX lease/watchdog and supervised qualification |
| M7 | Distinct app/binary/profile identity, inherited updater removed, pinned Windows GNU/native dependencies prepared | Full canonical build, unique candidate provenance, isolated GUI and installer acceptance |
| M8 | Reviewed three-platform CI workflow committed | Hosted jobs blocked by account billing/spending limit; actual platform builds and signing qualification |

Canonical xNexus `180fae465` integrates release isolation, the native client/outbox
and the first desktop slice. Completed commits are pushed immediately.
The first candidate version is reserved as `1.24.1-xlog.1`; no packaged candidate
is claimed here. See RELEASE_READINESS.md for the build gate.

## Coordination and constraints

The existing “xlog work” chat owns canonical xLog's startup corrections and
1.6.21 installation acceptance. Its dirty source and tests are preserved.
Bridge changes remain isolated until that owner reports the release boundary.
At approximately 10:43 UTC it requested a twenty-minute pause on new heavy
builds for timing measurements; existing builds could finish.

GitHub Actions was initially disabled with operator approval. Following audit,
the operator approved the validation workflow. Actions is enabled; inherited
release/deployment workflows are individually disabled and their source jobs
also require exact `kd9taw/Nexus` identity. No billing settings were changed.

[Validation run 43586230218](https://github.com/oliverbross/OM0RX-xNexus/actions/runs/43586230218)
failed before any job steps: GitHub reported account payment/spending-limit
restrictions. This is neither a test pass nor a source compilation failure.
Local Windows verification continues. macOS/Linux are unverified.

## Audio finding

xLog's existing recorder consumes native PCM, but its configuration identifies
a device and channel roles, not an authoritative radio/receiver association.
Its callback-time archive anchor does not certify sample-zero UTC accuracy for
digital-mode timing. No `audio.rx` capability is advertised from this source.
The missing work is explicit owner mapping, capture-clock calibration, bounded
timestamped PCM export and continuous normal-logging ownership. The application
must not guess that the selected radio owns an arbitrary headphone mix.

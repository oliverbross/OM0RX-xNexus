# Execution ledger — 14 November 2026

The full plan is being executed in separate desktop, xLog-owner and release
worktrees. This ledger distinguishes implemented source from release acceptance.

| Milestone | Current evidence | Remaining acceptance |
|---|---|---|
| M0 | Complete: full upstream history, private origin, fetch-only upstream, nine issues/milestones | Keep patch inventory and upstream review process current |
| M1 | 49 native client tests, Clippy/format pass; owner bridge compiles; actual HTTP/client and separate actual SQLite-owner restart test pass | Canonical xLog integration after its startup release; paired application acceptance |
| M2 | Separate default companion entry, native pairing and private data root; no local Engine, hardware, log or feeds; native all-targets check passed | Final native execution, packaging and isolated GUI acceptance |
| M3 | Authoritative owner needs, geography, band/world maps and selected-call history; shared Rust/TypeScript contract | Final owner projection lifecycle fixes, paired classification changes and full correctness matrix |
| M4 | Owner-mediated tune/point/independent Stop; identity/revision fences; saturation tests | Final native fake-owner UI and physical receive/control qualification |
| M5 | Durable outbox plus atomic SQLite write/receipt, restart reconciliation, edit-conflict test; recoverable workspace-bound draft | Final paired native GUI acceptance and operational queue archival design |
| M6 | Dormant FT8/FT4 decoder adapter and generated-message tests in source | Actual native decoder execution, mapped/calibrated xLog PCM, RX qualification, TX lease/watchdog and supervised qualification |
| M7 | Distinct app/profile/update identity, pinned native assets, exact-source packaging/provenance tooling | Final Windows native tests, unique-version package and isolated installer/application acceptance |
| M8 | Reviewed platform validation workflow; connected Mac build work started | Actual macOS/Linux results, signing and distribution qualification |

Canonical xNexus `d5cf11014` includes private draft recovery and workspace-bound
native enqueue. Final UI/client fixes and acceptance tooling are pushed on their
isolated branches for integration after the current native test build.
The first candidate version is reserved as `1.24.1-xlog.1`; no packaged candidate
is claimed here. See [VERIFICATION.md](VERIFICATION.md) for exact check boundaries.

## Coordination

The “xlog work” chat retains canonical xLog for startup corrections. Its 1.6.21
candidate failed live acceptance and was not installed; 1.6.22 is reserved for its
next startup candidate, not the companion bridge. Its unrelated changes remain
untouched. Bridge source stays in `codex/xnexus-bridge` until that owner reports a
clean successful installation boundary. Existing isolated owner tests do not
justify installing an unqualified production xLog build.

GitHub Actions was initially disabled with operator approval. Following audit,
the operator approved the validation workflow. Actions is enabled; inherited
release/deployment workflows remain individually disabled and guarded by exact
upstream repository identity. No billing setting changed.
[Run 43586230218](https://github.com/oliverbross/OM0RX-xNexus/actions/runs/43586230218)
was blocked before execution by payment/spending restrictions. Local Windows
verification continues; the connected Mac provides an alternative for macOS.

The upstream helper completed a real isolated merge rehearsal from pin
`c88740a94` to available upstream `3885df0fe`, preserving a separate review
worktree and leaving main unchanged. That pending merge is not a qualified
upstream upgrade; the shipped source pin remains unchanged.

## Audio and operational boundaries

xLog's existing recorder has neither authoritative radio/receiver mapping nor a
calibrated sample-zero UTC bound. No `audio.rx` capability is advertised. The
remaining work is explicit owner mapping, capture-clock calibration, bounded
PCM export and continuous normal-logging ownership. Transmission is unavailable
in companion mode. No on-air acceptance is implied by synthetic tests.

The native outbox retains up to 10,000 operations. It deliberately does not delete
terminal receipts automatically. An operator archive/export flow is a remaining
long-term operational item; keep this limit visible in the beta guide.

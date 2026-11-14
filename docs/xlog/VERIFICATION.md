# Integration verification — 14 November 2026

Latest native client source: `3a0686ee8`. Windows Rust 1.99.1 MSVC.
All 43 native client tests pass with the locked offline dependency graph, followed
by Clippy (`--all-targets -- -D warnings`) and rustfmt. This covers observation,
control receipts, independent Stop, durable file outbox recovery, explicit retry,
owner geography, and bounded selected-call history/confirmation reads. The
history tests prove portable-call URL encoding, no request without capability,
workspace fencing and refusal of invalid edit revisions.

The actual production bridge module from the isolated xLog source was compiled
with the actual production client in a temporary paired harness. Five tests
passed: the four then-current bridge transport cases plus authenticated pairing,
wrong-token refusal, tune receipt lookup, independent Stop, and revocation.
Its provider was synthetic: no physical device or production log was used.
The harness resides in the xLog workspace `.build/xnexus-paired-e2e`; eight
subsequent owner transport/projection cases are reported by the owner agent.

Fork identity/workflow policy: six tests pass on canonical xNexus. Upstream jobs
require the upstream repository identity, inherited workflows are disabled via
GitHub, and the fork updater cannot install upstream binaries. Windows GNU
toolchain and pinned libvpx, Hamlib and DeepCW assets are prepared. Native desktop
compilation is in progress; the first CMake attempt exposed a Windows verbatim
path issue, with a source correction awaiting the coordinated next build.

Desktop agent evidence: initial eight UI tests, then four QSO queue cases,
focused TypeScript validation, and two browser probe runs covering fourteen
viewport/theme combinations. Full desktop qualification and installed-app
acceptance remain open; browser mocks do not prove the Tauri/native wiring.

Hosted validation did not execute: GitHub run 43586230218 reports the account's
payment/spending restriction on every job. No macOS/Linux build is claimed.
No radio transmission, rotator movement, production log mutation or installation
has been performed by these verification tests.

## Archived foundation evidence

The following describes the earlier bootstrap, not current integration status.

Scope: `integrations/xlog`, a standalone read-only reference client. This is not
a compiled Nexus desktop or a qualified live xLog integration.

Environment: Windows x86_64 MSVC; rustc 1.99.1 (58cf8f9c3 2026-10-13),
cargo 1.99.1 (5f94df478 2026-08-27). Dependencies resolved offline from the
existing registry cache and pinned in integrations/xlog/Cargo.lock. Clippy was
installed for the existing toolchain; no desktop or station software installed.

Passed:
- `cargo test --manifest-path integrations/xlog/Cargo.toml --locked --offline`:
  20 library tests; zero failures and zero skips.
- `cargo clippy --manifest-path integrations/xlog/Cargo.toml --locked --offline --all-targets -- -D warnings`.
- `cargo fmt --manifest-path integrations/xlog/Cargo.toml --check`.

Tests verify exact authenticated GET routes; protocol/application/capability
admission; malformed and oversized replies (declared and streaming lengths);
redirect refusal with a second listener proving it received no credentials;
changed server/workspace; stale/future data; monotonic expiry; same-revision replay;
revision rollback; explicit reconnect; unknown need status; evidence-required
needed rows; duplicate identifiers; rotor/frequency limits; HTTP denial clearing
prior authority; and bounded refused connections. All data and sockets are synthetic.

Initial failures exposed the test server's accepted sockets inheriting Windows'
nonblocking mode. The server now sets each accepted socket blocking with a read
deadline. The final green run includes all initially failing cases. An initial
text replacement missed rustfmt's split method call; that attempt did not fix the
fixture, and its failed run is not counted as a pass.

Upstream `node scripts/gates --list --unrunnable` inventoried 92 gates and reported
68 unavailable in the initial shell. This is a discovery result, not execution.
The Rust binaries were then found outside PATH for the independent client checks.
The full upstream desktop/DSP/browser/platform suites have NOT run. No UI was
changed, launched, observed or packaged. No real xLog data, credentials, radio,
rotator, cluster, audio or production profile was accessed by these tests.

At that bootstrap, release blockers remained M1b–M8. In particular, upstream
identity, data directories, stable/beta updater and hosted services are still
present in the copied desktop. The current isolation status is recorded above.

The upstream helper passed PowerShell parser validation and a real report-only
fetch. It detected upstream advancing to
`6aadc44d9cc7bbbbd655c5265f466525054c3e31` (six commits after the inspected pin)
and performed no merge or source change. The prepared source remains pinned to
`c88740a94172b3e78ae587d62bd0a54394d1e85f`; review those newer changes in the first
upstream merge after bootstrap. The merge-preparation branch path has not been
exercised against this still-uncommitted bootstrap.
